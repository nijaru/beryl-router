# OpenWrt Custom Build Guide (GL-MT3000)

## Goal
Build a custom OpenWrt firmware image for the GL.iNet Beryl AX (GL-MT3000) that supports eBPF (BTF/CO-RE).

## Requirements
- `CONFIG_DEBUG_INFO_BTF=y` (Kernel)
- `CONFIG_BPF_SYSCALL=y` (Kernel)
- `pahole` (dwarves) > 1.16 available on build host (provided by Docker container)

## Build Environment

We use a Docker container to keep the host system clean.

```bash
# Build the container
docker build -t beryl-builder -f Dockerfile.openwrt .

# Run the container (mounting a build directory)
mkdir -p build
docker run -it --rm -v $(pwd)/build:/home/build/workspace beryl-builder
```

## Build Steps (Inside Container)

1. **Clone SDK/Source:**
   We use the GL.iNet MT7981 SDK or upstream OpenWrt (Filogic target).
   *Currently assuming upstream OpenWrt snapshot for best BPF support, or GL.iNet proprietary if WiFi drivers require it.*

   ```bash
   git clone https://git.openwrt.org/openwrt/openwrt.git
   cd openwrt
   ./scripts/feeds update -a
   ./scripts/feeds install -a
   ```

2. **Configuration:**
   ```bash
   make menuconfig
   ```
   - Target System: `MediaTek Ralink ARM`
   - Subtarget: `Filogic 8x0 (MT798x)`
   - Target Profile: `GL.iNet GL-MT3000`

3. **Kernel Configuration:**
   ```bash
   make kernel_menuconfig
   ```
   - `Kernel hacking` -> `Compile-time checks and compiler options` -> `Generate BTF typeinfo` (`CONFIG_DEBUG_INFO_BTF=y`)

4. **Build:**
   ```bash
   make -j$(nproc)
   ```

## Artifacts
The resulting image (sysupgrade.bin) will be in `bin/targets/mediatek/filogic/`.

## Flashing
See `ai/design/FLASHING.md` for U-Boot recovery instructions.
