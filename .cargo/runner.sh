#!/bin/bash
# Runner script for Raspberry Pi Pico 2 W (RP2350)

ELF_FILE="$1"
UF2_FILE="${ELF_FILE%.elf}.uf2"
TARGET_DRIVE="RP2350"
FAMILY="rp2350-arm-s"

echo "📦 Generating UF2 for: ${UF2_FILE##*/}"

# --- Generate UF2 ---
ELF_WITH_EXT="${ELF_FILE}.elf"
cp "$ELF_FILE" "$ELF_WITH_EXT" 2>/dev/null
picotool uf2 convert "$ELF_WITH_EXT" "$UF2_FILE" --family $FAMILY 2>/dev/null
rm -f "$ELF_WITH_EXT"

echo "⚡ Flashing..."

# 1. Try USB Drive Method (BOOTSEL mode)
if [ -d "/Volumes/$TARGET_DRIVE" ]; then
    echo "✅ $TARGET_DRIVE drive detected (BOOTSEL mode)"
    echo "📤 Copying UF2..."
    cp "$UF2_FILE" "/Volumes/$TARGET_DRIVE/"
    if [ $? -ne 0 ]; then 
        echo "❌ Failed to copy UF2 file. Ensure permissions are correct or drive is not busy."
        exit 1
    fi
    echo "🔄 Syncing..."
    sync
    echo "📤 Ejecting volume..."
    diskutil eject "/Volumes/$TARGET_DRIVE" >/dev/null 2>&1
    echo "✅ Flash complete! Device will reboot."
    sleep 2
    exit 0
fi

# 2. Check for device via picotool
INFO=$(picotool info -d 2>/dev/null)
if echo "$INFO" | grep -q "Chip: RP2350"; then
    echo "✅ RP2350 in BOOTSEL mode detected via picotool"
    echo "⚡ Loading..."
    picotool load -u -v -x -t elf "$ELF_FILE"
    if [ $? -ne 0 ]; then
        echo "❌ picotool load failed."
        exit 1
    fi
    echo "✅ Flash complete!"
    exit 0
elif echo "$INFO" | grep -q "Chip: RP2040"; then
    echo "❌ ERROR: Connected device is an RP2040 (Pico 1). This project is configured for RP2350 (Pico 2)."
    exit 1
fi

# 3. Try to force reboot into bootloader
echo "🔄 Checking for running device to reboot..."
if picotool reboot -f -u 2>/dev/null; then
    echo "✅ Reboot command sent. Waiting for bootloader..."
    sleep 3
    
    if [ -d "/Volumes/$TARGET_DRIVE" ]; then
        echo "✅ $TARGET_DRIVE mounted after reboot!"
        cp "$UF2_FILE" "/Volumes/$TARGET_DRIVE/"
        sync
        diskutil eject "/Volumes/$TARGET_DRIVE" >/dev/null 2>&1
        echo "✅ Flash complete!"
        exit 0
    fi

    # Try picotool load again
    if picotool load -u -v -x -t elf "$ELF_FILE" 2>/dev/null; then
        echo "✅ Flash complete via picotool after reboot!"
        exit 0
    fi
fi

# --- Failure Message ---
echo ""
echo "❌ FLASH FAILED"
echo ""
echo "💡 Manual Steps:"
echo "   1. Hold BOOTSEL button while plugging in the Pico 2 W."
echo "   2. Verify '/Volumes/RP2350' appears."
echo "   3. Run: 'cargo run --release'"
exit 1
