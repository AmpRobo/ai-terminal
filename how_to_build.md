# 编译 Linux 的 .deb 和 .AppImage

在 Linux 上打包图形界面客户端。两条产物共用一次 Rust 编译，再分别封装。

推荐直接运行：

```bash
./script/build_linux_packages
```

默认渠道是 `oss`。这个渠道不需要私有的 `ai-terminal-channel-config`。产物在：

```text
target/release-lto/bundle/linux/AI-Terminal-<arch>.AppImage
target/release-lto/bundle/linux/ai-terminal_<version>_<deb-arch>.deb
```

`<arch>` 为 `x86_64` 或 `aarch64`。`<deb-arch>` 对应为 `amd64` 或 `arm64`。未指定 `--release-tag` 时版本号是 `0.1.0`，deb 文件名类似 `ai-terminal_0.1.0_amd64.deb`。若环境里已经设置了 `CARGO_TARGET_DIR`，产物写到该目录下的同样相对路径。

## 脚本参数

```text
./script/build_linux_packages [--channel CHANNEL] [--packages LIST]
    [--release-tag TAG] [--debug] [--skip-build] [--arch ARCH]
    [--features FEATURES] [--install-system-deps]
```

| 参数 | 作用 |
| --- | --- |
| `--channel` | `oss`（默认）、`local`、`dev`、`preview`、`stable` |
| `--packages` | 逗号分隔，默认 `appimage,deb`。可以只写 `appimage` 或 `deb` |
| `--release-tag` | 传给 `GIT_RELEASE_TAG`。`.deb` 的 `Version` 会去掉前缀 `v`，并把 `preview_1` 这类后缀里的 `_` 换成 `.` |
| `--debug` | 使用 Cargo `dev` 配置，编译更快，二进制更大 |
| `--skip-build` | 跳过 `cargo build`，只用已有二进制重新打包 |
| `--arch` | `x86_64` 或 `aarch64`。GUI 二进制按本机架构编译，这里必须与本机一致 |
| `--features` | 追加到该渠道原有 Cargo features 之后 |
| `--install-system-deps` | 调用仓库里的 apt 安装脚本，并补装打包工具。需要 sudo |

`dev`、`local`、`preview`、`stable` 会在编译期嵌入渠道配置，PATH 上必须有 `ai-terminal-channel-config`（`./script/install_channel_config`，需要私有仓库权限）。没有该二进制时用 `oss`。

## 编译流程

`./script/build_linux_packages` 会先补齐不需要 root 的工具，再调用 `./script/bundle`。`script/bundle` 在 Linux 上转到 `script/linux/bundle`。下面是 GUI 包（`--artifact` 默认为 `app`）的实际顺序。

1. 选定渠道、Cargo profile 和包格式。
   - `oss`、`preview`、`stable` 使用 `release-lto`（thin LTO）。
   - `dev`、`local` 使用 `release-lto-debug_assertions`。
   - `--debug` 使用 `dev`。
2. 编译 GUI 二进制。`oss` 的命令等价于：

   ```bash
   cargo build -p ai-terminal --profile release-lto --bin ai-terminal-oss \
     --features release_bundle,gui,nld_classifier_v3,nld_heuristic_v2
   ```

   `dev` / `preview` / `stable` 还会带上 `crash_reporting` 等渠道 feature。首次 `release-lto` 构建通常要几十分钟。
3. 复制一份未 strip 的 `.debug` 二进制，再对要打包的二进制 `strip`。`dev` 只去掉调试符号，其余渠道去掉全部符号。
4. 运行编出的二进制 `dump-settings-schema`，生成 `settings_schema.json`。`--skip-build` 时必须能找到这份文件，或自行设置 `SETTINGS_SCHEMA_SOURCE` / `SETTINGS_SCHEMA_EXECUTABLE`。
5. 按 `--packages` 封装。两种格式都会先把二进制、`.desktop`、图标和 `resources/` 装进一个临时目录（`script/linux/bundle_install`）。`cargo about generate` 在这一步生成 `THIRD_PARTY_LICENSES.txt`，因此需要 `cargo-about` 0.8.4。

### AppImage

`script/linux/bundle_appimage` 调用 `linuxdeploy`：

- 放入可执行文件、desktop 文件和 PNG 图标。
- 使用 `script/linux/linuxdeploy-plugin-aiterminal`，把二进制挪到 `opt/aiterminal/<package>/`，并复制 bundled resources。
- `--output appimage` 交给 appimagetool 打成单个文件。
- `oss` 的文件名是 `AI-Terminal-<arch>.AppImage`。其他渠道用应用显示名，例如 `AiTerminalDev-x86_64.AppImage`。

`linuxdeploy` 本身也是 AppImage。没有 `/dev/fuse` 时脚本会设置 `APPIMAGE_EXTRACT_AND_RUN=1`，让它解压后运行。打包机还需要 `patchelf`。

### deb

`script/linux/bundle_deb`：

- 用 `resources/linux/debian/app/` 里的 control、postinst、postrm 模板填渠道、版本和架构。
- `oss` 不写入官方 apt 源配置。
- `fakeroot dpkg-deb -b` 生成 deb。
- `oss` 的包名是 `ai-terminal-oss`，文件名是 `ai-terminal_<version>_<deb-arch>.deb`。其他渠道的包名和文件名都带渠道后缀，例如 `ai-terminal-dev_0.1.0_amd64.deb`。`stable` 的包名和文件名都是 `ai-terminal`。

没有 `GIT_RELEASE_TAG` 时 control 的 `Version` 为空，`dpkg-deb` 会失败。脚本默认传入 `v0.1.0`。

## 不经过包装脚本时

在仓库根目录执行，效果与默认脚本相同：

```bash
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
./script/linux/install_linuxdeploy
./script/install_cargo_release_deps --no-build-deps
./script/bundle --channel oss --packages appimage,deb --release-tag v0.1.0
```

只打一种包时把 `--packages` 改成 `appimage` 或 `deb`。已有同 profile 的二进制时可以加 `--skip-build`，并保证对应的 `settings_schema.json` 还在，例如 `target/release-lto/settings_schema.json`。

## 系统依赖

Debian / Ubuntu 上，编译依赖由 `script/linux/install_build_deps` 安装：`build-essential`、`cmake`、`pkg-config`、`python-is-python3`、`libssl-dev`、`libfreetype-dev`、`libexpat1-dev`、`libgit2-dev`、`libfontconfig1-dev`、`libasound2-dev`、`jq`、`brotli`、`libclang-dev`、`curl`、`git`，以及 protoc 25.1。Rust 版本以仓库根目录的 `rust-toolchain.toml` 为准。

这个脚本不包含打包装进系统的工具。还需要：

```bash
sudo apt-get install -y fakeroot patchelf desktop-file-utils
```

`dpkg-deb` 来自系统自带的 `dpkg`。`--install-system-deps` 会走上述官方安装脚本并补上这三件打包工具。

`libgit2` 在 GUI 依赖里是 vendored 的，没有 `libgit2-dev` 也能编过。Linux 的 GUI 构建不会跑 macOS 那条 bindgen，因此没有 `libclang` 也能编这个包。官方安装脚本仍然会装它们。

## 安装和运行

```bash
sudo apt install ./target/release-lto/bundle/linux/ai-terminal_0.1.0_amd64.deb
./target/release-lto/bundle/linux/AI-Terminal-x86_64.AppImage
```

deb 的运行时依赖写在包控制文件里，包括 `fontconfig`、ALSA、EGL、Wayland、X11 和 `zlib1g`。AppImage 若提示无法用 FUSE 挂载：

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./target/release-lto/bundle/linux/AI-Terminal-x86_64.AppImage
```
