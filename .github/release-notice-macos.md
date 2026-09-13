## Installing on macOS

Download **`MicoldAIIDE-<version>-universal.dmg`** below. One file for both Apple silicon and Intel
Macs.

Open it, drag **Micold AI IDE** onto **Applications**, then eject the disk image. Open the app from
Applications, not from the disk image — a copy run from the image disappears when you eject it, and
the app will tell you so rather than pretend otherwise.

**The first launch is blocked, and that is expected.** macOS says it *"could not verify"* the
application. This project has no Apple Developer account, so the download is signed but names no
developer — macOS cannot attribute it to anyone and says so. Nothing is wrong with the file.

To open it:

1. Click **Done** on the dialog.
2. Open **System Settings › Privacy & Security**.
3. Scroll to **Security** and click **Open Anyway**.

That is the whole thing — once, in System Settings, no terminal. Every launch afterwards is silent.
A future release is a new download and is blocked once in the same way.

Full instructions, including what the app asks permission for and how to remove it:
[Installing on macOS](https://github.com/jaroslawherod/micold-ai-ide/blob/main/docs/user-guide/install-macos.md)
