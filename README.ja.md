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

パッケージ名は `lazykeifu`、起動コマンドは **`lzkf`** です。

lazykeifu は crates.io を通じて提供されています。ソースコードからビルドしてインストールするため、Rust/Cargo、Git、および C 言語のビルドツールチェーン（同梱される OpenSSL のビルドに必要な `make` や `Perl` を含む）が必要です。

Cargo を使用する場合:
```bash
cargo install lazykeifu --locked
```

mise を使用する場合:
```bash
mise use -g cargo:lazykeifu@0.1.1
```

インストール後、Git リポジトリのディレクトリへ移動して起動します。
```bash
cd path/to/repo
lzkf
```

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
