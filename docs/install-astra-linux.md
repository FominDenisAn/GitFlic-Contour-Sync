# Установка и запуск под Astra Linux

Перед установкой проверить архитектуру и runtime-зависимости:

```bash
uname -m
apt-cache policy libwebkit2gtk-4.1-0
apt-cache policy libgtk-3-0
```

## DEB

```bash
sudo apt install ./gitflic-contour-sync_*.deb
```

Если `apt` сообщает об отсутствующей `libwebkit2gtk-4.1-0`, сначала нужно решить совместимость WebKitGTK для этой версии Astra. Не следует обходить эту зависимость вручную.

## Portable binary

```bash
chmod +x ./GitFlic-Contour-Sync
./GitFlic-Contour-Sync
```

Portable binary всё равно использует системные GUI/runtime-библиотеки Linux.

## AppImage

Если AppImage был собран:

```bash
chmod +x ./*.AppImage
./*.AppImage
```

## Проверка SHA-256

```bash
sha256sum GitFlic-Contour-Sync
sha256sum *.deb *.AppImage 2>/dev/null
```

Сверить с `SHA256SUMS.txt`.
