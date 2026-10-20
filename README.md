# lazykeifu（仮称）

keifuのGitグラフを土台に、lazygitの操作感を目指すRust製TUIクライアントです。作業ファイル・ブランチ・コミットグラフ・差分を一画面で操作する最初の版を実装しています。

- [現在の操作と対応範囲](docs/workspace-m1.md)
- [当面の開発計画](docs/development-plan.md) / [初期化の検証結果](docs/bootstrap-validation.md)
- [設定](docs/configuration.md) / [TUIデバッグ](docs/debugging.md)
- [上流keifuの操作説明](docs/upstream/README.md)
- [参照用lazygit fork](https://github.com/watabegg/lazygit)

```sh
cargo build --locked
# 対象のGitリポジトリ内で実行
/path/to/lazykeifu/target/debug/lazykeifu
```

設定はOSの設定ディレクトリ内の `lazykeifu/config.toml`（Linuxでは通常 `~/.config/lazykeifu/config.toml`）、ログレベルは `LAZYKEIFU_LOG` を使用します。既存のkeifuの設定は読みません。現在の既定動作では60秒ごとにGit設定に従ってfetchします。変更方法は設定ドキュメントを参照してください。

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
# Linuxで実TUIを検証（scriptコマンドが必要）
cargo build --locked
python3 scripts/test-workspace.py
```

[keifu](https://github.com/trasta298/keifu)の履歴とMITライセンスを引き継いでいます。lazygitは操作仕様の参照元で、現段階ではコードを取り込んでいません。名称と配布方針が固まるまでCargo publishと自動リリースを停止しています。

現在の同期操作: `f`でfetch、`p`でpull、`P`でpush。追跡先・`pushRemote`・pull方針などはGit設定に従います。pull／push前には確認画面を表示し、設定不足や競合はGitのエラーを表示します。pullによる自動マージのメッセージはGitの既定値を使用します。

`s`で作業ツリーをstashに保存します（未追跡ファイルを含み、ignoredファイルは除外）。`5`のstash一覧では`Space`でapply、`p`でpop、`d`でdrop、`Enter`で差分へ移動します。apply／popはGitの既定動作と同じく元のstage状態を復元しません。競合したpopではstashを残します。外部でstashを変更したときは`R`で一覧を更新してください。

ファイル一覧の`d`で未stageの変更を確認後に破棄します（indexは保持、未追跡ファイルは削除）。差分ペインでは`[`／`]`でhunkを選び、`Space`でstage／unstageします。hunk操作は変更済みのUTF-8通常ファイルに対応し、新規・削除・rename・モード変更・バイナリはファイル単位で操作します。表示後に差分が変わった場合は`R`で再確認してください。

Branches（`3`）またはGraph（`4`）でブランチを選び、`M`で現在のブランチへmerge、`r`で現在のブランチを選択先へrebaseします。確認画面に方向と対象を表示し、Git本体のエディタ・フック・署名を使って実行します。未コミット変更の自動stashは行いません。競合時はファイルを解消して`C`で続行、`A`で中止できます。

作業ファイルの`e`でGitのエディタ設定（`GIT_EDITOR`／`core.editor`など）を使って編集し、終了後に再読込します。GUIエディタは`code --wait`のように終了を待つ設定が必要です。競合ファイルの`Space`は確認後に現在の内容を解消済みとしてstageします（競合マーカーの意味判断は行いません）。merge／rebase中は`C`で続行、`A`で中止を確認後に実行できます。続行時はGitのエディタ・フック・署名にも端末を渡します。外部エディタの起動には`sh`を使います（WindowsではGit付属の`sh`をPATHに追加する必要があります）。
