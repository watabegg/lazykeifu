# ワークスペース M1 利用・引継ぎメモ

本資料（`docs/workspace-m1.md`）は、Rust 版 keifu フォークの初回統合ワークスペースにおける実装済み機能と未対応事項を整理した利用・引継ぎメモである。マイルストーン 1（M1）としての初回操作可能版であり、現状の動作状況と制約を記録する。

## 画面構成と基本操作

左ペインに status、作業ファイル、branches、右ペインに graph と diff を配置する。幅 80 未満または高さ 18 未満ではフォーカス中のペインのみを描画し、フッターは常時表示する。
Tab、Shift+Tab、h、l でフォーカスを巡回し、数字キー（`1`: status、`2`: 作業ファイル、`3`: branches、`4`: graph、`0`: diff）で直接切り替える（`2` でコミット履歴から作業ファイルへ復帰可能）。ペイン内は j/k 移動、Ctrl-d/u ページ移動、g/G 端移動、H/L 横スクロールに対応する。マウスによる選択、ホイール、ドラッグコピー、ファイルや graph のダブルクリックが可能である。`?` でヘルプを表示し、`q` で終了する。

## 実装済みの機能

### 履歴と差分の閲覧
graph でコミットを選択すると commit files を左に表示し、Enter でその一覧へ、ファイル上で Enter を押すと diff へ遷移する。履歴参照時の Space は読み取り専用を通知し、Esc で作業ツリー（worktree）へ戻る。作業ファイルと commit files は旧 50 件上限を廃止して全件到達可能としたが、graph 履歴の 500 件上限は維持する。
diff はバックグラウンドで取得し、commit、path、staged 側・unstaged 側、revision の変更時に古い結果を破棄する。差分は行単位の色分け表示に対応するが、以前の構文や単語単位の強調表示（syntax/word highlighting）は新ワークスペースでは未接続である。

### ステージングとコミット
Space で stage/unstage を行う。index と作業ツリー双方に変更があるファイルは、unstaged 側の Space でそのファイル全体を stage し、staged 側の Space で unstage する。v で staged/unstaged を切り替え、作業ツリー表示時のみ a で全 stage、u で全 unstage を行う（hunk や行単位の stage は未対応）。
c でメッセージを入力し Enter でコミットを実行する。Git CLI 経由で実行し hook や署名設定を尊重する。ワーカースレッド実行中は追加操作や終了を抑制し、失敗時は入力を維持して Git 出力を表示し Enter で再試行できる。非対話実行（標準入力 null）のため対話的 hook や端末入力署名は未対応であり、失敗時に出力を確認する。agent 等による非対話署名は Git 設定に従って動作する。

### ブランチ操作と内部構造
branches フォーカス時は `/` で検索、`b` で作成、`d` で削除（未マージを拒否する `git branch -d`）を行い、Space または Enter でチェックアウトする。既存ローカルとリモートの OID が異なる場合はリモートチェックアウトを拒否しローカルをリセットしない。origin 以外のリモート追跡にも対応する。f で fetch、P で push を実行できるが既存 origin 固定である。未対応の p（pull）と s（stash）は通知を表示し、誤って push へ割り当てないよう保護する。
状態と遷移は `src/workspace.rs`、描画は `src/ui/workspace.rs`、CLI 実行（リテラルパス指定や index キャッシュ再読み込み）は `src/git/command.rs`、キー対応は `src/keybindings.rs` で管理する。旧 CLI の `--file-view` オプション削除に伴い設定 `ui.file_view` は無効となった。旧 `FileSelect`/`FileDiff` 描画コードは内部に残るが、通常操作では新ワークスペースを使用する。

## 検証方法と再現手順

現時点で Rust の各種テストや fmt、clippy、Python 実 TUI テストによる検証が完了している。最終変更後にも再実行できるよう、以下の再現コマンドを中心に記載する。

```bash
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
python3 scripts/test-workspace.py
```

- **Python テスト**（Linux の `script` コマンドが必要）: 一時リポジトリで 61 ファイル、部分 index、hook 失敗・再試行・入力維持、ビジーガード、履歴読み取り専用保護、極小画面（20x6、54x20）、連続高速選択、初回コミット、リンクされた worktree の HEAD/index 隔離を検証。
- **Rust テスト**: 記号付きファイル名 `[x]`、名前変更ファイルの stage/unstage、乖離したリモートの拒否、未コミット変更時のチェックアウト保護、変更済み状態で Ctrl-Space が stage しないリグレッション防止を検証。

## 次の段階と制限事項

本実装は M1 の初回操作可能版であり、M2 で upstream を尊重したネットワーク操作、stash、hunk stage、変更破棄（discard）確認を実装し、M3 で競合解消や rebase 等に対応する。
現段階の制限事項として、graph 履歴 500 件上限、origin 固定ネットワーク、diff 構文・単語強調の未接続、旧 file-view 廃止、対話的 hook や対話的署名の不可が挙げられる。また、リポジトリ再読み込み、一覧取得、一部 Git 操作は同期実行であり、巨大リポジトリでの応答性目標は未測定である。Windows および macOS 環境はローカル未検証である。
