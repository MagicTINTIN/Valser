package fr.magictintin.valser

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Build
import android.os.Handler
import android.os.Looper

object Bridge {
  init {
    System.loadLibrary("valser_lib")
  }

  @JvmStatic external fun attach()
  @JvmStatic external fun command(name: String, value: Long)

  /** Called from the Rust playback thread, state is 0 stopped, 1 playing, 2 paused. */
  @JvmStatic fun update(state: Int, title: String, artist: String, duration: Long, position: Long) {
    MediaService.instance?.let { service ->
      service.handler.post { service.update(state, title, artist, duration, position) }
    }
  }
}

class MediaService : Service() {
  companion object {
    var instance: MediaService? = null
    private const val CHANNEL = "playback"
    private const val NOTIFICATION = 1
  }

  val handler = Handler(Looper.getMainLooper())
  private lateinit var session: MediaSession
  private var state = -1
  private var title = ""
  private var artist = ""
  private var duration = -1L

  override fun onCreate() {
    super.onCreate()
    session = MediaSession(this, "Valser")
    session.setCallback(object : MediaSession.Callback() {
      override fun onPlay() = Bridge.command("play", 0)
      override fun onPause() = Bridge.command("pause", 0)
      override fun onSkipToNext() = Bridge.command("next", 0)
      override fun onSkipToPrevious() = Bridge.command("prev", 0)
      override fun onStop() = Bridge.command("stop", 0)
      override fun onSeekTo(position: Long) = Bridge.command("seek", position)
    })
    session.isActive = true

    getSystemService(NotificationManager::class.java)
      .createNotificationChannel(NotificationChannel(CHANNEL, "Playback", NotificationManager.IMPORTANCE_LOW))
    if (Build.VERSION.SDK_INT >= 29) {
      startForeground(NOTIFICATION, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
    } else {
      startForeground(NOTIFICATION, notification())
    }
    instance = this
    Bridge.attach()
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    intent?.action?.let { Bridge.command(it, 0) }
    return START_NOT_STICKY
  }

  override fun onBind(intent: Intent?) = null

  override fun onDestroy() {
    instance = null
    session.release()
    super.onDestroy()
  }

  fun update(state: Int, title: String, artist: String, duration: Long, position: Long) {
    val playing = state == 1
    session.setPlaybackState(
      PlaybackState.Builder()
        .setActions(
          PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or PlaybackState.ACTION_PLAY_PAUSE or
            PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS or
            PlaybackState.ACTION_STOP or PlaybackState.ACTION_SEEK_TO
        )
        .setState(
          when (state) {
            1 -> PlaybackState.STATE_PLAYING
            2 -> PlaybackState.STATE_PAUSED
            else -> PlaybackState.STATE_STOPPED
          },
          position,
          if (playing) 1f else 0f
        )
        .build()
    )
    if (state == this.state && title == this.title && artist == this.artist && duration == this.duration) return

    this.state = state
    this.title = title
    this.artist = artist
    this.duration = duration
    session.setMetadata(
      MediaMetadata.Builder()
        .putString(MediaMetadata.METADATA_KEY_TITLE, title)
        .putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
        .putLong(MediaMetadata.METADATA_KEY_DURATION, duration)
        .build()
    )
    getSystemService(NotificationManager::class.java).notify(NOTIFICATION, notification())
  }

  private fun action(icon: Int, label: String, name: String): Notification.Action {
    val intent = Intent(this, MediaService::class.java).setAction(name)
    val pending = PendingIntent.getService(this, name.hashCode(), intent, PendingIntent.FLAG_IMMUTABLE)
    return Notification.Action.Builder(icon, label, pending).build()
  }

  private fun notification(): Notification {
    val open = PendingIntent.getActivity(
      this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE
    )
    val toggle = if (state == 1) {
      action(android.R.drawable.ic_media_pause, "Pause", "pause")
    } else {
      action(android.R.drawable.ic_media_play, "Play", "play")
    }
    return Notification.Builder(this, CHANNEL)
      .setSmallIcon(android.R.drawable.ic_media_play)
      .setContentTitle(title.ifEmpty { "Valser" })
      .setContentText(artist)
      .setContentIntent(open)
      .setOngoing(state == 1)
      .addAction(action(android.R.drawable.ic_media_previous, "Previous", "prev"))
      .addAction(toggle)
      .addAction(action(android.R.drawable.ic_media_next, "Next", "next"))
      .setStyle(Notification.MediaStyle().setMediaSession(session.sessionToken).setShowActionsInCompactView(0, 1, 2))
      .build()
  }
}
