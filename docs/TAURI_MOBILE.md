# Getting the Tauri mobile app onto your phone

This doc explains the **different ways** to run and distribute the STG Tauri app on a real phone (Android and iOS). The app is the same Yew frontend in a Tauri shell; you build it from `front/tauri`.

---

## Prerequisites (both platforms)

- **Tauri CLI**: `cargo install tauri-cli`
- **Backend**: For a real device, the app must reach your API. Options:
  - Your backend running on a machine on the **same Wi‑Fi** as the phone, and you use that machine’s LAN IP in the app (e.g. `http://192.168.1.10:50002`).
  - A **deployed** backend (e.g. production URL). Configure the Yew app / Trunk build to use that base URL for API calls when building the mobile app.

---

## Android

### 1. Run on device in development (fastest for testing)

You install and run the app **once** from your PC; it stays on the phone until you uninstall or run again.

**Requirements:**

- [Android SDK](https://developer.android.com/studio) (or Android Studio) installed.
- Phone: **Developer options** and **USB debugging** enabled, connected via USB.
- Optional: [Android NDK](https://developer.android.com/ndk) if the Tauri Android build needs it (Tauri docs will mention it).

**Steps:**

```bash
cd front/tauri
cargo tauri android dev
```

- First run can take a while (Gradle, NDK, etc.).
- The app is built, installed on the connected device (or emulator), and launched. You can disconnect the USB cable after launch; the app keeps running until closed.
- To update: run `cargo tauri android dev` again; it reinstalls and launches.

**How you get it on the phone:** The CLI installs it over USB via `adb`. No APK file to copy; the app is pushed and started from your machine.

---

### 2. Build an APK and install it yourself (sideload)

You produce a single **APK** file and install it on your (or someone else’s) phone without the Play Store.

**Requirements:**

- Android SDK (and NDK if required by Tauri).
- For **release** builds, you should [sign the APK](https://v2.tauri.app/distribute/sign/android) (keystore, etc.). For quick testing you can use a **debug** build (see below).

**Steps:**

```bash
cd front/tauri
# Release APK (signed; requires signing config in src-tauri/gen/android/...)
cargo tauri android build --apk

# Or debug APK (no signing; for quick testing on your own device)
cargo tauri android build --apk --debug
```

- Output is under `front/tauri/src-tauri/gen/android/app/build/outputs/apk/` (e.g. `universalRelease` or `universalDebug`). The file is usually named like `app-universal-release.apk` or `app-universal-debug.apk`.

**Ways to get that APK onto the phone:**

1. **USB**: Copy the APK to the phone (e.g. `adb push path/to/app.apk /sdcard/Download/`), then on the phone open the file and tap “Install”. Or install directly: `adb install path/to/app.apk`.
2. **Cloud / link**: Upload the APK to Google Drive, Dropbox, or your own server; open the download link on the phone and install (you may need to allow “Install from unknown sources” for that browser or file manager).
3. **Email / messaging**: Send the APK to yourself and open it on the phone; then install.

**Limitation:** Sideloaded APKs don’t auto-update; you distribute new builds manually (same steps again).

---

### 3. Google Play Store (internal testing, closed/open testing, production)

For distribution to many users and optional auto-updates.

**Requirements:**

- [Google Play Developer account](https://play.google.com/console/signup) (one-time fee).
- [Android app signing](https://v2.tauri.app/distribute/sign/android) set up (upload key, then Play App Signing).
- Build an **AAB** (Android App Bundle), not just an APK.

**Steps:**

```bash
cd front/tauri
cargo tauri android build
# Produces AAB under gen/android/app/build/outputs/bundle/... (e.g. release).
```

- In [Google Play Console](https://play.google.com/console) create an app, then upload the AAB to a **release track**:
  - **Internal testing**: up to 100 testers by email; quickest way to get a “store-like” install (testers get a link to join and download).
  - **Closed testing** or **Open testing**: broader testers or public beta.
  - **Production**: full public release.

**How testers/users get the app on their phone:** They use the Play Store (or the testing link from Play Console). No manual APK handling.

---

## iOS

### 1. Run in the simulator (no phone, Mac only)

Good for UI and flow testing without a device.

**Requirements:**

- **Mac** with **Xcode** installed.
- Tauri iOS toolchain (see [Tauri – Mobile development](https://v2.tauri.app/start/mobile/)).

**Steps:**

```bash
cd front/tauri
cargo tauri ios dev
```

- Choose the simulator when prompted (or it uses a default). The app runs in the simulator only; **not** on a physical phone.

---

### 2. Run on your iPhone/iPad (development)

You install and run the app on your own device from your Mac.

**Requirements:**

- **Mac** with **Xcode**.
- **Apple Developer account** (free account can install on your own device for a limited time; paid [$99/year] is needed for longer and for distribution).
- Device connected via USB (or same Apple ID / network for wireless debugging).
- In Xcode: add your Apple ID (Settings → Accounts), and set the app’s **Team** and **Signing** so the device is trusted.

**Steps:**

```bash
cd front/tauri
cargo tauri ios dev
```

- When multiple targets exist (simulator vs device), select the **physical device**. The app is built, signed with your development certificate, installed on the device, and launched.

**How you get it on the phone:** The CLI builds and installs over the connection (USB or network). No separate file to copy; same idea as Android `android dev`.

---

### 3. TestFlight (internal and external testers)

Best way to put the app on **other people’s** iPhones/iPads without publishing to the App Store.

**Requirements:**

- **Apple Developer Program** ($99/year).
- [App Store Connect](https://appstoreconnect.apple.com) app created; [iOS code signing](https://v2.tauri.app/distribute/sign/ios) set up (certificate, provisioning profile).

**Steps:**

1. Build an **IPA** (archive) for distribution. From the Tauri project you typically open the generated Xcode project and use **Product → Archive**, or use Tauri’s iOS build and then archive in Xcode:
   ```bash
   cd front/tauri
   cargo tauri ios build --open
   ```
   Then in Xcode: Archive → Distribute App → **App Store Connect** → Upload.
2. In **App Store Connect** → your app → **TestFlight**: add **Internal** testers (same team) and/or **External** testers (up to 10,000; first build needs a short Apple review).
3. Testers get an email or open the **TestFlight** app on their iPhone, accept the invite, and install.

**How testers get the app on their phone:** They use the TestFlight app (or link) and install from there. No manual IPA handling.

---

### 4. App Store (public release)

For public distribution and optional auto-updates.

**Requirements:**

- Same as TestFlight (Developer Program, code signing, App Store Connect).
- App Store listing (screenshots, description, privacy, etc.).

**Steps:**

- After uploading a build (same as TestFlight), in App Store Connect you submit that build for **App Review** and choose **Release** (manual or automatic). Once approved, the app is available on the App Store.

**How users get the app:** Search or link in the App Store → Install. No manual files.

---

## Summary table

| Goal | Android | iOS |
|------|--------|-----|
| **Quick test on your device** | `cargo tauri android dev` (USB) | `cargo tauri ios dev` (Mac + device) |
| **Single APK/IPA, install yourself** | `cargo tauri android build --apk` → copy / adb install or link | Build IPA → install via Xcode to device (or TestFlight for yourself) |
| **Share with testers** | Internal testing track in Play Console (AAB) or send APK | TestFlight (upload IPA, invite testers) |
| **Public store** | Play Store (AAB, Play Console) | App Store (IPA, App Store Connect) |

---

## Backend URL for mobile builds

The Yew frontend in `front/web` calls the API using a base URL (e.g. from `config` or env). For **desktop** dev, that’s usually `http://localhost:50002`. For **mobile**:

- **Same Wi‑Fi**: Build (or configure at runtime if you add a settings screen) to use your dev machine’s LAN IP, e.g. `http://192.168.1.10:50002`.
- **Production**: Point the mobile build at your deployed API (e.g. `https://api.yourdomain.com`).

Right now the base URL is likely fixed at build time (Trunk/env). To support both dev and prod mobile, you’d add a build-time or runtime config (e.g. env var or a small config screen in the app). That’s a separate step from “getting the app on the phone.”

---

For Tauri’s own prerequisites and tool versions, see the official [Tauri – Mobile development](https://v2.tauri.app/start/mobile/) and [Distribute](https://v2.tauri.app/distribute/) docs.
