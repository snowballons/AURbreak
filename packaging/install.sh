#!/usr/bin/env bash
set -euo pipefail

PREFIX="/usr/local"
if [[ "${1:-}" == "--user" ]]; then
    PREFIX="${HOME}/.local"
fi

BINDIR="${PREFIX}/bin"
APPDIR="${PREFIX}/share/applications"
ICONDIR="${PREFIX}/share/icons/hicolor/scalable/apps"

echo "Installing AURbreak to ${PREFIX}..."
mkdir -p "${BINDIR}" "${APPDIR}" "${ICONDIR}"

install -m 755 aurbreak "${BINDIR}/aurbreak"
ln -sf "${BINDIR}/aurbreak" "${BINDIR}/browse-arch"

if [[ -f packaging/aurbreak.desktop ]]; then
    install -m 644 packaging/aurbreak.desktop "${APPDIR}/aurbreak.desktop"
fi

if [[ -f ui/assets/aurbreak-icon-app.svg ]]; then
    install -m 644 ui/assets/aurbreak-icon-app.svg "${ICONDIR}/aurbreak.svg"
elif [[ -f aurbreak.svg ]]; then
    install -m 644 aurbreak.svg "${ICONDIR}/aurbreak.svg"
fi

# Update desktop database and icon cache if available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APPDIR}" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" || true
fi

echo "AURbreak installed successfully!"
echo "Run 'aurbreak' or launch it from your application menu."
