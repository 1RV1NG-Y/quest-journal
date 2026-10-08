# Local workflow

After changing the desktop app, build and update the installed version using the desktop installer. The user tests the installed application, so source changes and frontend builds alone do not complete the work. Verify the installed binary matches the new build and tell the user if restarting is needed. Preserve existing user data.

The browser native host must be copied to its permanent installed location. Never point browser manifests or launch wrappers at `target/debug` or `target/release`. Verify the installed wrapper with a framed native-messaging request; for Brave, also check it inside the Flatpak runtime. Run `node scripts/test-native-install.mjs` after building the release native host to guard against build-directory cleanup breaking the extension.
