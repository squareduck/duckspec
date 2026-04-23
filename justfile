# Install ds and duckboard to ~/.cargo/bin.
install:
    cargo install --path crates/duckspec
    cargo install --path crates/duckboard

# Push main bookmark to origin (jj).
push:
    jj bookmark set main -r @-
    jj git push --bookmark main

# Fetch from origin (jj).
fetch:
    jj git fetch

# Cut a new release: bump workspace version, commit, push, tag.
# Pushing the tag triggers .github/workflows/release.yml which builds
# Duckboard.dmg on a macOS runner and attaches it to the release.
#
# Usage: just release 0.2.0
release version:
    #!/usr/bin/env bash
    set -euo pipefail

    # Refuse to run on a dirty working copy — `cargo set-version` would
    # otherwise get folded into unrelated in-flight edits.
    if [ -n "$(jj diff)" ]; then
        echo "Error: working copy has uncommitted changes. Commit or abandon them first."
        exit 1
    fi

    # Treat warnings as errors for the release build. Catches dead code,
    # unused imports, and the like *before* the tag goes out — a broken
    # CI run on a cut release is awkward to undo.
    echo "==> Checking release build (warnings as errors)"
    RUSTFLAGS="-D warnings" cargo check --release --workspace

    cargo set-version --workspace {{ version }}

    jj commit -m "release: v{{ version }}"
    jj bookmark set main -r @-
    jj git push --bookmark main

    # jj doesn't manage tags; push the tag directly via git so the
    # release workflow picks it up.
    git tag "v{{ version }}"
    git push origin "v{{ version }}"

    echo ""
    echo "🦆 Release v{{ version }} triggered. Watch CI at:"
    echo "   https://github.com/squareduck/duckspec/actions"

# Build a macOS Duckboard.app bundle at dist/Duckboard.app.
bundle:
    #!/usr/bin/env bash
    set -euo pipefail

    APP_NAME="Duckboard"
    BUNDLE_ID="cc.squaredu.duckboard"
    VERSION=$(cargo pkgid -p duckboard | awk -F'#' '{print $NF}')
    SRC_ICON="crates/duckboard/assets/app-icon.png"
    OUT_DIR="dist"
    APP="${OUT_DIR}/${APP_NAME}.app"
    CONTENTS="${APP}/Contents"

    echo "==> Building release binary (duckboard ${VERSION})"
    cargo build --release -p duckboard

    echo "==> Assembling ${APP}"
    rm -rf "${APP}"
    mkdir -p "${CONTENTS}/MacOS" "${CONTENTS}/Resources"
    cp target/release/duckboard "${CONTENTS}/MacOS/duckboard"

    echo "==> Building .icns from ${SRC_ICON}"
    ICONSET=$(mktemp -d)/${APP_NAME}.iconset
    mkdir -p "${ICONSET}"
    # macOS iconset expects these 10 PNG sizes. Keys are pixel dimensions,
    # values are the iconset filenames that should be produced at that size.
    declare -a mappings=(
        "16:icon_16x16.png"
        "32:icon_16x16@2x.png"
        "32:icon_32x32.png"
        "64:icon_32x32@2x.png"
        "128:icon_128x128.png"
        "256:icon_128x128@2x.png"
        "256:icon_256x256.png"
        "512:icon_256x256@2x.png"
        "512:icon_512x512.png"
        "1024:icon_512x512@2x.png"
    )
    for m in "${mappings[@]}"; do
        size="${m%%:*}"
        name="${m##*:}"
        sips -z "$size" "$size" "${SRC_ICON}" -o "${ICONSET}/$name" >/dev/null
    done
    iconutil -c icns "${ICONSET}" -o "${CONTENTS}/Resources/${APP_NAME}.icns"

    echo "==> Writing Info.plist"
    cat > "${CONTENTS}/Info.plist" <<PLIST
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>CFBundleName</key>
        <string>${APP_NAME}</string>
        <key>CFBundleDisplayName</key>
        <string>${APP_NAME}</string>
        <key>CFBundleIdentifier</key>
        <string>${BUNDLE_ID}</string>
        <key>CFBundleVersion</key>
        <string>${VERSION}</string>
        <key>CFBundleShortVersionString</key>
        <string>${VERSION}</string>
        <key>CFBundleExecutable</key>
        <string>duckboard</string>
        <key>CFBundlePackageType</key>
        <string>APPL</string>
        <key>CFBundleIconFile</key>
        <string>${APP_NAME}</string>
        <key>LSMinimumSystemVersion</key>
        <string>11.0</string>
        <key>NSHighResolutionCapable</key>
        <true/>
        <key>NSPrincipalClass</key>
        <string>NSApplication</string>
    </dict>
    </plist>
    PLIST

    echo "==> Built ${APP}"

# Wrap Duckboard.app in a drag-to-Applications DMG at dist/Duckboard.dmg.
bundle-dmg: bundle
    #!/usr/bin/env bash
    set -euo pipefail

    APP_NAME="Duckboard"
    VERSION=$(cargo pkgid -p duckboard | awk -F'#' '{print $NF}')
    OUT_DIR="dist"
    APP="${OUT_DIR}/${APP_NAME}.app"
    DMG="${OUT_DIR}/${APP_NAME}-${VERSION}.dmg"

    STAGE=$(mktemp -d)/"${APP_NAME}"
    mkdir -p "${STAGE}"
    cp -R "${APP}" "${STAGE}/"
    # Drag-to-install target inside the DMG.
    ln -s /Applications "${STAGE}/Applications"

    echo "==> Building ${DMG}"
    rm -f "${DMG}"
    hdiutil create \
        -volname "${APP_NAME}" \
        -srcfolder "${STAGE}" \
        -ov \
        -format UDZO \
        "${DMG}" >/dev/null

    echo "==> Built ${DMG}"
