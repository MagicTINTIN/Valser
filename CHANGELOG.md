<!-- LTeX: language=en-US -->
# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [\<1.0.1\>](https://github.com/MagicTINTIN/Valser/releases/v1.0.1) - 2026-11-13 <small><small><small>[(compare)](https://github.com/MagicTINTIN/Valser/compare/v1.0.0..v1.0.1)</small></small></small>

### 🚜 Refactor

- changing android version too (in tauri) <small><small>[(cb0c0b7)](https://github.com/MagicTINTIN/Valser/commit/cb0c0b714a2e603d452040ea2ebee3c4be7c26b2)</small></small>

---
## [\<1.0.0\>](https://github.com/MagicTINTIN/Valser/releases/v1.0.0) - 2026-11-13 <small><small><small>[(compare)](https://github.com/MagicTINTIN/Valser/compare/v0.2.2..v1.0.0)</small></small></small>

### 🚀 Features

- valser android build (the app exists) + signature setup <small><small>[(47b3f51)](https://github.com/MagicTINTIN/Valser/commit/47b3f51e43618abba4c3200b5e26379e282bfde3)</small></small>
- adding music activity for android <small><small>[(e6cae18)](https://github.com/MagicTINTIN/Valser/commit/e6cae1828db9e0035d93bf337c1ad904bad35399)</small></small>
- adding basic equalizer <small><small>[(7e56b14)](https://github.com/MagicTINTIN/Valser/commit/7e56b14075e52ea3fcfe038b1cee5b0e9ca4f7ce)</small></small>

### 🐛 Bug Fixes

- file picker undefined functions <small><small>[(e6572c6)](https://github.com/MagicTINTIN/Valser/commit/e6572c6ca2c9392994ef474f81c7a70836179208)</small></small>
- build for android and switching opus lib bc deprecated <small><small>[(d216de9)](https://github.com/MagicTINTIN/Valser/commit/d216de972e8ef08d15adbb0599beccd9a3fa89d6)</small></small>
- the app now starts on android <small><small>[(cdd0f70)](https://github.com/MagicTINTIN/Valser/commit/cdd0f70dd485a56df02178cdd20f4dd37256a2db)</small></small>
- sink error when unplugging headphone <small><small>[(eef93cf)](https://github.com/MagicTINTIN/Valser/commit/eef93cf9e8cd4ba7c501392969186ec37404f6f3)</small></small>
- title status <small><small>[(7fe8f65)](https://github.com/MagicTINTIN/Valser/commit/7fe8f65390bd209af10324a74c66defa8ef7a50c)</small></small>
- not empty query <small><small>[(2f59c67)](https://github.com/MagicTINTIN/Valser/commit/2f59c671396d9eabfb41acaab4e0dcc9662c9b21)</small></small>
- missing builder for windows <small><small>[(7eaaa37)](https://github.com/MagicTINTIN/Valser/commit/7eaaa3717212693053e17167dec20a9ef632dce8)</small></small>

### ⚡ Performance

- displaying only visible rows <small><small>[(e32aa3b)](https://github.com/MagicTINTIN/Valser/commit/e32aa3b12ef097b57d1da97e470fc4101205a8e8)</small></small>

### 🚜 Refactor

- splitting to valser and valser-android, adding dependencies for android <small><small>[(0dfbe88)](https://github.com/MagicTINTIN/Valser/commit/0dfbe88823384f560799abe4a4ca0d7d726879ba)</small></small>
-  **⚠️ breaking!** switch to tauri (#16) <small>[**\[#16\]**](https://github.com/MagicTINTIN/Valser/issues/16)</small> <small><small>[(385b5d1)](https://github.com/MagicTINTIN/Valser/commit/385b5d1289ed6f56db3cc0e3a17aef25f2ccb620)</small></small>
- prev/next now skips hidden tracks <small><small>[(d32baf0)](https://github.com/MagicTINTIN/Valser/commit/d32baf0ca24ab3e38e3dae881b54714ea2b1bfde)</small></small>
- adding versionning with git cliff <small><small>[(59ec4ed)](https://github.com/MagicTINTIN/Valser/commit/59ec4edcb8cd1ad312f7f45271761ac57bde742e)</small></small>

### ⚙️ Miscellaneous Chores

- **(release)** 1.0.0 <small><small>[(d5e33f3)](https://github.com/MagicTINTIN/Valser/commit/d5e33f3b1751b170c7d3e3844dcb6528077b3142)</small></small>

### 🎨 Style

- adding logo <small><small>[(3890e32)](https://github.com/MagicTINTIN/Valser/commit/3890e3247430e95465900eca2b05c7034c5a0aba)</small></small>
- adding new icon and theme <small><small>[(bff0384)](https://github.com/MagicTINTIN/Valser/commit/bff03843b69a3d97c5019b08a51a0d0f69b7bced)</small></small>
- adding some icons and changing displaed name <small><small>[(b44f5cf)](https://github.com/MagicTINTIN/Valser/commit/b44f5cf3be1871377b04e426024cf22acb3d0973)</small></small>
- current music index + equalizer size <small><small>[(4f5b0f1)](https://github.com/MagicTINTIN/Valser/commit/4f5b0f1e99eff714e0133ba47ad42935cdd86fea)</small></small>

### 🏗️ Build

- disabling simd for windows arm build <small><small>[(c3fb3a3)](https://github.com/MagicTINTIN/Valser/commit/c3fb3a3dc71f6a01f4f5badedabefdaf2410cefd)</small></small>

---
## [\<0.2.2\>](https://github.com/MagicTINTIN/Valser/releases/v0.2.2) - 2026-07-06 <small><small><small>[(compare)](https://github.com/MagicTINTIN/Valser/compare/v0.2.1..v0.2.2)</small></small></small>

### 🚀 Features

- save state each 5s <small><small>[(29c541f)](https://github.com/MagicTINTIN/Valser/commit/29c541f7dad741ddd1cbf0a50aebaba7651afe7b)</small></small>
- saving states on actions <small>[**\[#14\]**](https://github.com/MagicTINTIN/Valser/issues/14)</small> <small><small>[(4593b88)](https://github.com/MagicTINTIN/Valser/commit/4593b889929ffd0fd02770da4eb3acb9c345de85)</small></small>
- clear entire playlist <small><small>[(cc3bc1e)](https://github.com/MagicTINTIN/Valser/commit/cc3bc1e0c721e49c1098faf818e36ccb7c3ffe77)</small></small>

### ⚡ Performance

- improving playlist scrollpane <small><small>[(fde9cb4)](https://github.com/MagicTINTIN/Valser/commit/fde9cb43e36f8c8b11d3a4624870cba8c9273934)</small></small>
- parallel loader for heavy number of tracks <small><small>[(7d5a757)](https://github.com/MagicTINTIN/Valser/commit/7d5a7572983e40b1a3ea391bd2d350747f96ab54)</small></small>
- cap fps to 30 when focused and less on low power <small><small>[(71c446a)](https://github.com/MagicTINTIN/Valser/commit/71c446a2df4c3cb4bd621a4767b91a9258eab4d3)</small></small>
- caching genre count <small><small>[(7a671f5)](https://github.com/MagicTINTIN/Valser/commit/7a671f526dcdd895a7050cc25ac53c86881d1ef6)</small></small>

### ⚙️ Miscellaneous Chores

- **(release)** 0.2.2 <small><small>[(2adaf4a)](https://github.com/MagicTINTIN/Valser/commit/2adaf4aadb94126502d42da2676902c253fb9e8f)</small></small>
- adding configuration definitions and locations <small><small>[(a692794)](https://github.com/MagicTINTIN/Valser/commit/a69279496bd80d96814607a59d4e5e68ceda0b8e)</small></small>

### 🎨 Style

- unused settings path fix <small><small>[(f0f4e77)](https://github.com/MagicTINTIN/Valser/commit/f0f4e7783a9060c15fe8502a29d5391cba54f9f7)</small></small>

---
## [\<0.2.1\>](https://github.com/MagicTINTIN/Valser/releases/v0.2.1) - 2026-06-30 <small><small><small>[(compare)](https://github.com/MagicTINTIN/Valser/compare/v0.2.0..v0.2.1)</small></small></small>

### 🐛 Bug Fixes

- using pre release hook of git cliff <small><small>[(2154694)](https://github.com/MagicTINTIN/Valser/commit/21546942beccbcef80e160cbc6840b983a437361)</small></small>

### ⚙️ Miscellaneous Chores

- **(release)** 0.2.1 <small><small>[(fd590b9)](https://github.com/MagicTINTIN/Valser/commit/fd590b938f88f02ca8f7ee12ccd3377a77abdb6b)</small></small>

### 🏗️ Build

- adding a script to bump version <small><small>[(a709626)](https://github.com/MagicTINTIN/Valser/commit/a709626728873f5010d037847abc90e22ba3397f)</small></small>

---
## [\<0.2.0\>](https://github.com/MagicTINTIN/Valser/releases/v0.2.0) - 2026-06-30 <small><small><small>[(compare)](https://github.com/MagicTINTIN/Valser/compare/v0.0.1..v0.2.0)</small></small></small>

### 🚀 Features

- new UI and mp3 openable using rodio (#11) <small>[**\[#11\]**](https://github.com/MagicTINTIN/Valser/issues/11)</small> <small><small>[(d41533f)](https://github.com/MagicTINTIN/Valser/commit/d41533fde83fa7d30e647866cc201da64e3f54ab)</small></small>
- opus compatibility (#12) <small>[**\[#12\]**](https://github.com/MagicTINTIN/Valser/issues/12)</small> <small><small>[(d62e68a)](https://github.com/MagicTINTIN/Valser/commit/d62e68a4a18d5c4f02ba380c10bd34549b652235)</small></small>
- implement seek on opus source <small><small>[(07a372a)](https://github.com/MagicTINTIN/Valser/commit/07a372aabfc1962f51865275740c7a4c163a4aaf)</small></small>
- adding directory in playlist, then shuffle it <small><small>[(da26130)](https://github.com/MagicTINTIN/Valser/commit/da261304cf60cf66b5f910d55b686f07789e047f)</small></small>
- adding some keyboard shortcuts <small><small>[(21eeae9)](https://github.com/MagicTINTIN/Valser/commit/21eeae9b58214fd975d4bdd44669feb144548c54)</small></small>
- reading music tags using lofty <small><small>[(8e78f1c)](https://github.com/MagicTINTIN/Valser/commit/8e78f1ca4d8d41b612910bd1af11d278fddce5cf)</small></small>
- adding tag filtering and add directory actions <small><small>[(1c23439)](https://github.com/MagicTINTIN/Valser/commit/1c23439eb5e50fac5f3ece58dab6655ed6edaafb)</small></small>
- adding keys to navigate in the music <small><small>[(ba5c5e3)](https://github.com/MagicTINTIN/Valser/commit/ba5c5e3816b979f26e3dc3bfbc347c041c0cbabe)</small></small>
- control using xf86/mpris <small>[**\[#1\]**](https://github.com/MagicTINTIN/Valser/issues/1)</small> <small><small>[(02516c8)](https://github.com/MagicTINTIN/Valser/commit/02516c818cc45f4c57faf00f3d57a3806c136869)</small></small>
- adding connect_seek with updated status while playing <small><small>[(4900197)](https://github.com/MagicTINTIN/Valser/commit/49001971effb5a7360f32152ce99c4ba11f6f721)</small></small>

### ⚙️ Miscellaneous Chores

- adding egui default example <small><small>[(6ebb56d)](https://github.com/MagicTINTIN/Valser/commit/6ebb56dc0d281390407c67752ab42d754cedbc1a)</small></small>
- Release valser version 0.2.0 <small><small>[(42e625c)](https://github.com/MagicTINTIN/Valser/commit/42e625c29a967b73f2bac7ef525a3db6bce4865f)</small></small>

### 📚 Documentation

- adding cargo toml missing fields <small><small>[(c6ca9b4)](https://github.com/MagicTINTIN/Valser/commit/c6ca9b4f1e5c6f7697a1b71655a7c27b18901c8d)</small></small>
- adding changelog <small><small>[(353edd2)](https://github.com/MagicTINTIN/Valser/commit/353edd2b8d44da31608aa2f94a2dd580a0dc46bb)</small></small>

### 🎨 Style

- adding custom colors and slider size <small><small>[(27ee560)](https://github.com/MagicTINTIN/Valser/commit/27ee560f7a08792a3a1221cdb4787f8904e0bfe2)</small></small>

### 🏗️ Build

- nixos and rust toolchain <small><small>[(976353d)](https://github.com/MagicTINTIN/Valser/commit/976353d364b13f5b677c1d0b2a816e91052f662d)</small></small>

---
## [\<0.0.1\>](https://github.com/MagicTINTIN/Valser/releases/v0.0.1) - 2025-07-05

### ⚙️ Miscellaneous Chores

- initialising rust project <small><small>[(527e2d8)](https://github.com/MagicTINTIN/Valser/commit/527e2d8a7ba0af62da8f3e3a1de7c285620a8ec9)</small></small>

<!-- generated by git-cliff -->
