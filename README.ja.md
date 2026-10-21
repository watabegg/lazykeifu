# lazykeifu

[English README](https://github.com/watabegg/lazykeifu/blob/main/README.md)

[![Crates.io](https://img.shields.io/crates/v/lazykeifu.svg)](https://crates.io/crates/lazykeifu)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/watabegg/lazykeifu/blob/main/LICENSE)

![lazykeifu の画面概要](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/overview.png)
*検証用リポジトリを用いた実際の動作画面*

lazykeifuは、Rust 製の Git 向けターミナルユーザーインターフェース（TUI）です。[keifu](https://github.com/trasta298/keifu) のグラフ描画を基盤とし、[lazygit](https://github.com/jesseduffield/lazygit) に着想を得たキーボード主体の操作感を採用しています。keifu から派生して開発されている独立したプロジェクトであり、lazygit の完全な代替を意図したものではありません。

作業中も常に Git グラフが表示され、変更差分は必要なときにだけ手元で開いて確認できます。

## 主な機能

### ファイルツリーとコミットグラフ
折りたたみ可能なファイルツリーと色分けされた変更状態により、作業ツリーの状況を直感的に把握できます。Files、Worktrees、Submodules の各タブや、Local、Remotes、Tags の各タブを切り替えて操作可能です。画面下部には状況に応じたキー操作が案内され、コミット詳細パネルではファイルごとの追加・削除行数を確認できます。

チェックアウト、ブランチの作成や名前変更、タグ作成、スタッシュ、フェッチ・プル・プッシュ、マージ・リベース、チェリーピックやリバートなど、日常的な Git 操作に対応しています。

### 差分表示とステージング
![lazykeifu の差分表示](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/diff.png)

Files パネルで `Enter` を押すと差分が開き、`Esc` で閉じます。構文ハイライト、行番号、単語単位の変更箇所が表示されます。ファイル全体のステージングに加えて、変更された UTF-8 テキストであればハンク単位でのステージングにも対応しています。

### コミットダイアログ
![lazykeifu のコミット画面](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/commit.png)

Files パネルで `Space` を押して対象をステージし、`c` でコミットダイアログを開きます。メッセージ入力欄の横に、ステージされたファイル一覧と対象の差分が並んで表示されます。ステージされたファイルがない状態で `c` を押した場合は、すべての変更をステージするか確認が表示されます。

ダイアログ内では `Tab` で入力項目や一覧を移動し、件名欄で `Enter` を押すか `Ctrl-s` を入力してコミットを実行します。`Esc` でダイアログを閉じても、同一セッション内であれば入力中の下書きが保持されます。

## 主なキー操作

| キー | 動作 |
| --- | --- |
| `2` | Files パネルへ移動 |
| `3` | Branches パネルへ移動 |
| `4` | Graph パネルへ移動 |
| `5` | Stashes パネルへ移動 |
| `6` | Commit detail パネルへ移動 |
| `Tab` | フォーカスの移動 |
| `[` / `]` | パネル内タブの切り替え |
| `Space` | Files パネルでステージ / アンステージ |
| `Enter` | Files や Graph パネルで差分を表示 |
| `c` | Files パネルでコミットダイアログを開く |
| `?` | キーバインドのヘルプを表示 |

## インストール

`lazykeifu`（実行バイナリ名: `lzkf`）のビルド済みアーカイブは、[GitHub Releases](https://github.com/watabegg/lazykeifu/releases) から入手できます。なお、本プロジェクトのリリースタグには `lazykeifu-v0.1.2` のように `lazykeifu-vVERSION` という形式が使われます。リポジトリ内に存在する `v*` 形式のタグは上流である keifu の履歴を保持したものであり、本プロジェクトのリリースを示すものではありません。

### ビルド済みバイナリの利用

以下の環境向けにビルド済みアーカイブが提供されています。

- Linux x86_64（Ubuntu 22.04 / glibc 2.35 以降）
- Linux aarch64（Ubuntu 24.04 / glibc 2.39 以降）
- macOS x86_64 および aarch64（macOS 14 以降を基準）
- Windows x86_64

アーカイブ名は `lzkf-0.1.2-TARGET.tar.gz`（Windows は `.zip`）です。各アーカイブにはバイナリ本体に加え、`LICENSE`、`LICENSE.lazygit`、`THIRD_PARTY.json`、`README.md` が同梱されます。また、リリースアセットとして `SHA256SUMS` が提供されます。

バイナリの実行には Git がインストールされ PATH に通っている必要がありますが、Rust や C コンパイラは不要です。ただし、Windows 環境でエディタを起動するには Git for Windows に同梱されている `sh` が PATH に通っている必要があります。macOS 向けバイナリは署名や公証が行われていない点にご留意ください。

利用環境の OS とアーキテクチャに対応したアーカイブを展開し、`lzkf`（Windows では `lzkf.exe`）を PATH の通ったディレクトリに配置してください。Linux や macOS では、展開後に次のコマンドで配置できます（`~/.local/bin` が PATH に含まれていることを確認してください）。

```bash
mkdir -p ~/.local/bin
install -m755 lzkf ~/.local/bin/lzkf
```

### ソースコードからのインストール

ソースコードからビルドしてインストールする場合は、Cargo または mise を利用できます。

```bash
cargo install lazykeifu --locked
```

または

```bash
mise use -g cargo:lazykeifu@0.1.2
```

ソースビルドを行うには、Rust/Cargo と Git に加え、同梱される OpenSSL のビルドに必要な make や Perl などのネイティブビルドツールが必要です。mise を利用する場合も cargo バックエンドでビルドされるため、バイナリが直接ダウンロードされるわけではありません。また、mise では公開からの経過期間ポリシーによって最新版が `latest` から除外される場合があるため、バージョンを固定して指定します。

### 起動と不具合報告

インストール後、操作対象の Git リポジトリへ移動して起動します。

```bash
cd path/to/repo
lzkf
```

不具合が発生した場合は、[GitHub Issues](https://github.com/watabegg/lazykeifu/issues) へ報告してください。報告の際は、バージョン、OS、使用している端末（ターミナル）、再現手順を記載してください。

## 多言語対応

9 言語の UI に対応しており、ターミナルのロケール設定に応じて自動的に言語が選択されます。起動オプションで明示的に指定することも可能です。

```bash
lzkf --language ja
```

設定項目の詳細は [設定ドキュメント](https://github.com/watabegg/lazykeifu/blob/main/docs/configuration.md) を、詳しい操作手順は [利用ガイド（日本語）](https://github.com/watabegg/lazykeifu/blob/main/docs/usage.md) を参照してください。

## ライセンスと謝辞

本プロジェクトは [MIT License](https://github.com/watabegg/lazykeifu/blob/main/LICENSE) のもとで公開されています。

- グラフ描画および初期コードの基盤として [keifu](https://github.com/trasta298/keifu) を利用しています。
- 操作感の着想および一部の翻訳リソースとして [lazygit](https://github.com/jesseduffield/lazygit) を参考にしています。原著作者の表示を保持しており、翻訳リソースのライセンスは [LICENSE.lazygit](https://github.com/watabegg/lazykeifu/blob/main/locales/LICENSE.lazygit) に記載されています。
