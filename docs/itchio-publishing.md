# itch.io publishing

## Page description

The formatted page-description source is maintained in [`itchio-description.docx`](itchio-description.docx). Section titles use Word's Heading 2 style, corresponding to itch.io's recommended Header 2 formatting. The document is intended as a copy-and-paste source for the page's rich-text description editor; HTML source is not required.

## GitHub Actions publishing

The Android workflow mirrors complete GitHub Releases to itch.io with Butler. It waits for the five desktop archives from the CI/CD workflow as well as the three Android APKs before publishing, preventing the two release workflows from racing each other.

The following repository-level Actions settings control the integration:

- **Variable `ITCHIO_TARGET`** — the itch.io target in `username/game-slug` form. For a page such as `https://username.itch.io/game-slug`, the value is `username/game-slug`.
- **Secret `BUTLER_API_KEY`** — an itch.io API key supplied to Butler through the `BUTLER_API_KEY` environment variable. The secret remains in GitHub Actions and is not stored in repository files or workflow output.

When either setting is absent, the Android release workflow skips the itch.io mirror and continues uploading the APKs to the GitHub Release. When both settings are present, tag-based releases are mirrored automatically.

## itch.io channels

Each release asset is published to its own channel:

| Release asset | Butler channel |
| --- | --- |
| Linux x86_64 archive | `linux-x86_64` |
| Linux aarch64 archive | `linux-aarch64` |
| macOS aarch64 archive | `macos-aarch64` |
| Windows x86_64 archive | `windows-x86_64` |
| Windows aarch64 archive | `windows-aarch64` |
| Android x86_64 APK | `android-x86_64` |
| Android aarch64 APK | `android-aarch64` |
| Android ARMv7 APK | `android-armv7` |

Butler uses channel names to infer platform tags. It does not currently provide architecture tags, so architectures are kept in separate channel names for clarity. The Android builds are uploaded as APK files rather than renamed archives.

The GitHub release tag is passed to Butler as the user version for every channel.

## Re-publishing an existing release

A full release rebuild uses the CI/CD workflow with `operation=release` and the Android APK workflow with `action=release`, with the same release tag and appropriate source branch. The Android publish job performs the itch.io mirror after the desktop and Android assets are available on that GitHub Release.

When desktop assets are already present and only the Android APKs need to be rebuilt, the Android workflow's `release` operation is sufficient.

## Butler reference

- [Butler manual](https://itch.io/docs/butler/)
- [Running Butler in CI](https://itch.io/docs/butler/login.html)
- [Pushing builds](https://itch.io/docs/butler/pushing.html)

The workflow downloads the stable Linux x86_64 Butler build from itch.io's permanent [Broth](https://broth.itch.zone/butler) endpoint.
