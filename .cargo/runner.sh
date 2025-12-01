#!/bin/bash
# Automatic runner that generates UF2 and flashes

ELF_FILE="$1"
UF2_FILE="${ELF_FILE%.elf}.uf2"

# Generate UF2 with picotool (supports RP2350 correctly)
echo "📦 Generating UF2 for RP2350: ${UF2_FILE##*/}"

# Copy ELF with extension for picotool
ELF_WITH_EXT="${ELF_FILE}.elf"
cp "$ELF_FILE" "$ELF_WITH_EXT" 2>/dev/null

# Generate UF2 with correct family ID for RP2350
picotool uf2 convert "$ELF_WITH_EXT" "$UF2_FILE" --family rp2350-arm-s 2>/dev/null

# Clean up temporary file
rm -f "$ELF_WITH_EXT"

echo "⚡ Flashing..."

# Check if already in bootloader mode (via USB drive)
if [ -d "/Volumes/RP2350" ]; then
    echo "✅ Pico is already in bootloader (drive mounted)"
    echo "📤 Copying UF2..."
    cp "$UF2_FILE" /Volumes/RP2350/
    echo "🔄 Syncing..."
    sync
    echo "📤 Ejecting volume..."
    diskutil eject /Volumes/RP2350 2>/dev/null
    echo "✅ Flash complete! Pico will reboot."
    sleep 2
    exit 0
fi

# Check if in bootloader via picotool
if picotool info -d 2>/dev/null | grep -q "in BOOTSEL mode"; then
    echo "✅ Pico in bootloader, flashing..."
    picotool load -u -v -x -t elf "$ELF_FILE"
    exit 0
fi

# Pico is running firmware - forcing reboot to bootloader
echo "🔄 Pico detected running firmware"
echo "   Forcing reboot in bootloader mode..."

if picotool reboot -f -u 2>/dev/null; then
    echo "✅ Reboot sent, waiting for bootloader..."
    sleep 3

    # Try flash after reboot
    if picotool load -u -v -x -t elf "$ELF_FILE" 2>/dev/null; then
        echo "✅ Flash complete!"
        exit 0
    fi
fi

# If we got here, something went wrong
echo ""
echo "⚠️  Could not flash automatically"
echo ""
echo "🔍 Checking connection..."
ls /dev/cu.usbmodem* 2>/dev/null && echo "   ✅ Pico connected via USB serial" || echo "   ❌ Pico not detected"
echo ""
echo "💡 Solutions:"
echo "   1. Try disconnecting and reconnecting the Pico"
echo "   2. Or manually enter bootloader:"
echo "      - Hold BOOTSEL"
echo "      - Connect USB (or RESET)"
echo "      - Release BOOTSEL"
echo "      - Run: cargo run --release"
exit 1