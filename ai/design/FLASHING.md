# Flashing & Development Setup

## GL-MT3000 (Beryl AX) Flash Methods

### Method 1: Web UI (Easiest)

Stock GL-iNet firmware has an upgrade page.

1. Build OpenWrt image (`.bin` file)
2. Connect to Beryl via WiFi or LAN
3. Go to `http://192.168.8.1` → System → Upgrade
4. Upload the `.bin` file
5. Wait for reboot (~2-3 min)

**Limitation:** Only works if current firmware is functional.

### Method 2: U-Boot Recovery (Recommended for Dev)

Works even if firmware is bricked.

1. Connect PC to Beryl **LAN port** via Ethernet
2. Set PC to static IP: `192.168.1.2/24`
3. Hold reset button while powering on Beryl
4. Keep holding for ~10 seconds until LED flashes
5. Release - Beryl enters U-Boot recovery mode
6. Browse to `http://192.168.1.1`
7. Upload firmware image
8. Wait for flash + reboot

**This is your safety net** - you can always recover.

### Method 3: Serial Console (UART)

For debugging or if U-Boot is corrupted.

1. Open Beryl case (voids warranty)
2. Connect USB-UART adapter to debug pads:
   - TX → RX (3.3V logic)
   - RX → TX
   - GND → GND
3. Serial settings: 115200 8N1
4. Access U-Boot console
5. Use TFTP to load image

**Only needed if U-Boot itself is broken** (rare).

## Development Connection Scenarios

### Scenario A: Behind Existing Network (Your Setup)

```
Internet → Xfinity Box → Asus AP (192.168.1.x)
                              │
                              ├── Your Mac (192.168.1.10)
                              │
                              └── Beryl WAN (192.168.1.50)
                                      │
                                  Beryl LAN (192.168.8.x)
                                      │
                                  Test Client (192.168.8.100)
```

**Setup:**
1. Connect Beryl WAN port to Asus AP (or switch)
2. Beryl gets IP from Asus (e.g., 192.168.1.50)
3. SSH from Mac: `ssh root@192.168.1.50`
4. Connect test device to Beryl LAN/WiFi for testing

**Pros:** Easy, isolated test environment
**Cons:** Double NAT if testing outbound

### Scenario B: Beryl as AP (Simpler Testing)

```
Internet → Xfinity Box → Asus AP (192.168.1.x)
                              │
                              ├── Your Mac (192.168.1.10)
                              │
                              └── Beryl (Bridge mode)
                                      │
                                  Test Client (192.168.1.100)
                                  (IP from Asus DHCP)
```

**Setup:**
1. Configure Beryl in AP/Bridge mode
2. All devices on same subnet
3. Beryl just does WiFi AP + filtering

**Pros:** No NAT complexity, easy testing
**Cons:** Less isolated

### Scenario C: Direct Connection (Isolated Dev)

```
Mac ←──────── Ethernet ────────→ Beryl LAN
(192.168.8.2)                  (192.168.8.1)
```

**Setup:**
1. Connect Mac directly to Beryl LAN port
2. Mac gets IP from Beryl DHCP (or set static)
3. No internet on Beryl (WAN disconnected)

**Pros:** Fully isolated, no network interference
**Cons:** No internet access for testing upstream

## Development Workflow

### Initial Setup (One Time)

```bash
# On your Mac - add SSH config
cat >> ~/.ssh/config << 'EOF'
Host beryl
    HostName 192.168.1.50  # Adjust to actual IP
    User root
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
EOF

# Test connection
ssh beryl "uname -a"
```

### Deploy Cycle

```bash
# On Fedora (nick@fedora) - build
cd ~/beryl-router
cargo xtask build --release

# Deploy to router
scp target/aarch64-unknown-linux-musl/release/beryl-routerd beryl:/tmp/
ssh beryl "/tmp/beryl-routerd --help"

# Or use rsync for faster iterations
rsync -avz --progress \
  target/aarch64-unknown-linux-musl/release/beryl-routerd \
  beryl:/tmp/
```

### Testing

```bash
# SSH into Beryl
ssh beryl

# Run daemon in foreground (see logs)
/tmp/beryl-routerd --interface eth0 --config /etc/beryl/config.toml

# In another terminal, test API
curl http://192.168.8.1:8080/api/v1/status

# Watch packet stats
watch -n1 'curl -s http://192.168.8.1:8080/api/v1/stats'
```

## Building Custom OpenWrt Image

### Prerequisites

```bash
# On Fedora
sudo dnf install -y \
  gcc gcc-c++ binutils patch bzip2 flex bison make autoconf \
  gettext texinfo unzip sharutils ncurses-devel zlib-devel \
  gawk openssl-devel libxslt wget python3

# Clone OpenWrt
git clone https://git.openwrt.org/openwrt/openwrt.git
cd openwrt
git checkout v23.05.3  # Or latest stable
```

### Configure for Beryl AX + BTF

```bash
# Update feeds
./scripts/feeds update -a
./scripts/feeds install -a

# Start config
make menuconfig
```

**Required Settings:**

```
Target System: MediaTek Ralink ARM
Subtarget: MT7981
Target Profile: GL.iNet GL-MT3000

Global Build Settings:
  [*] Compile the kernel with debug info
      [*] Generate BTF type information  # CRITICAL

Kernel modules > Network Support:
  [*] kmod-sched-bpf

# Remove bloat (optional but recommended)
LuCI: [ ] (deselect all)
Base system:
  [ ] dnsmasq  # We're replacing it
```

### Build

```bash
# Download sources
make download -j8

# Build (takes 1-2 hours first time)
make -j$(nproc)

# Image location
ls bin/targets/mediatek/mt7981/openwrt-*-gl-mt3000-squashfs-sysupgrade.bin
```

### Flash Custom Image

```bash
# Copy to router
scp bin/targets/mediatek/mt7981/openwrt-*-sysupgrade.bin beryl:/tmp/

# SSH and flash
ssh beryl
sysupgrade -v /tmp/openwrt-*-sysupgrade.bin

# Or use U-Boot recovery if nervous
```

## Safety Checklist

- [ ] Know U-Boot recovery procedure BEFORE flashing custom firmware
- [ ] Keep a copy of stock firmware (download from GL-iNet)
- [ ] Test SSH access after each flash before continuing
- [ ] Don't flash while connected only via WiFi (use Ethernet)
- [ ] Have serial adapter ready (optional but recommended)
