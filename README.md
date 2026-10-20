# pedigit

A lightweight Rust Git TUI combining keifu's graph with lazygit-inspired workflows.

pedigit（pedigree + Git）は、Rust 製の軽量な Git 向け TUI です。[keifu](https://github.com/trasta298/keifu) v0.7.0 の Git グラフとコミット詳細表示を基盤とし、[lazygit](https://github.com/jesseduffield/lazygit) に着想を得たキー操作と作業ペインによって日常の Git 操作を完結させます。

## 特徴

- **常設グラフと詳細表示**: 常時表示される Git グラフで履歴の全体像を把握しながら、選択したコミットの詳細情報を確認できます。
- **ペイン内タブとファイルツリー**: Files / Worktrees / Submodules や Local / Remotes / Tags のペイン内タブを備え、変更ファイルは折り畳み可能なツリー形式で整理して扱えます。
- **差分確認とコミットダイアログ**: ステージ済み差分を確認できるコミットダイアログを備え、diff 表示では行番号、構文の色分け、単語単位の変更強調に対応しています。
- **日常操作の完結と多言語対応**: fetch、pull、push、stash や merge、rebase、cherry-pick、revert、競合時の続行・中止に対応しています。UI は 9 言語（en, ja, ko, nl, pl, pt, ru, zh-CN, zh-TW）をサポートします。

## インストール

現在はプレリリース段階のため、ソースコードからのビルドのみ提供しています。配布手順の整備が完了するまで、Cargo への公開停止（`publish = false`）および自動リリースの停止を維持します。

リポジトリ: [watabegg/pedigit](https://github.com/watabegg/pedigit)

```bash
git clone https://github.com/watabegg/pedigit.git
cd pedigit
cargo build --release --locked
```

ビルド後、対象リポジトリ内で生成されたバイナリを実行します。

```bash
/path/to/pedigit/target/release/pedigit
```

## 設定

設定ファイルは OS の設定ディレクトリ（Linux の場合は通常 `~/.config/pedigit/config.toml`）に配置します。表示言語は設定ファイルの `ui.language = "ja"` や CLI オプション `--language ja` で指定でき、ログ出力の制御には環境変数 `PEDIGIT_LOG` を使用します。

旧設定ファイルや旧環境変数からの引き継ぎは行いません。設定やログ出力の制御には、`pedigit` の設定ファイルおよび環境変数を使用してください。

## ドキュメント

詳細な情報は以下のドキュメントを参照してください。

- 操作方法: [docs/usage.md](docs/usage.md)
- 設定一覧: [docs/configuration.md](docs/configuration.md)
- 開発計画: [docs/development-plan.md](docs/development-plan.md)
- デバッグ: [docs/debugging.md](docs/debugging.md)

開発時のコード検証:

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

## 出自と謝辞

本製品は [keifu](https://github.com/trasta298/keifu) の履歴および著作権を保持して開発を継続している独立したフォークです。また、操作体系の設計や一部の翻訳において [lazygit](https://github.com/jesseduffield/lazygit) を参照・利用しています。

- ソフトウェアライセンス: [MIT License](LICENSE) (Copyright (c) 2025 trasta298, Copyright (c) 2026 watabegg)
- 共通翻訳ライセンス: [locales/LICENSE.lazygit](locales/LICENSE.lazygit) (Copyright (c) 2018 Jesse Duffield)
- 上流との差分の基点を追跡できるよう、keifu のベースコミット（[015f0b25](https://github.com/trasta298/keifu/commit/015f0b25b85d068b1e2c8c0c8c137bc87d4b7c91)）への固定リンクを設けています。過去のコミット履歴の改変や削除は行っていません。
- [keifu の基点から pedigit で加えた変更](https://github.com/watabegg/pedigit/compare/015f0b25b85d068b1e2c8c0c8c137bc87d4b7c91...main)
