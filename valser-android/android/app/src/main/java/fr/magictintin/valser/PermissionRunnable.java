package fr.magictintin.valser;

import android.app.Activity;

public class PermissionRunnable implements Runnable {
    private final Activity activity;
    private final String[] permissions;
    private final int requestCode;

    public PermissionRunnable(Activity activity, String[] permissions, int requestCode) {
        this.activity = activity;
        this.permissions = permissions;
        this.requestCode = requestCode;
    }

    @Override
    public void run() {
        activity.requestPermissions(permissions, requestCode);
    }
}