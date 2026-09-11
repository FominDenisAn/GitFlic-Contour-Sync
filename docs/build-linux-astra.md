# Сборка под Linux / Astra Linux

Linux-сборку лучше выполнять на Linux-системе, максимально близкой к целевой Astra Linux. Не рекомендуется собирать Linux-бинарник на более новой системе и считать его совместимым без проверки: версия glibc и WebKitGTK влияет на переносимость.

## 1. Проверить Astra Linux

На целевой или тестовой Astra:

```bash
cat /etc/os-release
uname -m
ldd --version | head -1
apt-cache policy libwebkit2gtk-4.1-0
apt-cache policy libgtk-3-0
```

Tauri 2 требует WebKitGTK 4.1. Если пакет отсутствует в репозиториях конкретной версии Astra, текущую Tauri 2 сборку нельзя считать совместимой без отдельной адаптации.

## 2. Зависимости машины сборки

Для Debian-подобной системы Tauri рекомендует development-пакеты WebKitGTK 4.1, GTK и стандартный toolchain. Типовой набор:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Также нужны Node.js/npm и Rust/Cargo.

Проверка:

```bash
node --version
npm --version
rustc --version
cargo --version
```

## 3. Сборка

```bash
cd /path/to/gitflic-contour-sync
npm install
chmod +x ./Build-Linux.sh ./scripts/Build-Linux.sh
./Build-Linux.sh
```

## 4. Результат

```text
artifacts/linux/
├── GitFlic-Contour-Sync
├── *.deb
├── *.AppImage
└── SHA256SUMS.txt
```

Наличие `.deb` и `.AppImage` зависит от того, какие bundle targets Tauri смог собрать на конкретной Linux-системе.

Для Astra предпочтительно сначала проверить `.deb`, затем portable binary/AppImage.
