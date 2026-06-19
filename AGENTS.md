# loom

loom はマークダウンライクなテキストベース作曲用の DSL である

必要に応じて、docs/llms.md を参照して loom を理解すること

## DevOps

- mise でツールを管理する
- just でタスクを管理する
- 実装が終了後、 `just ci` コマンドで品質チェックをする
    - ドキュメントのみの変更の場合は不要
- コミットメッセージは `type: 日本語の概要` の形式にする
    - type は Conventional Commits に従う
- コード探索や影響調査は MCP の `code-review-graph` を使う
    - トークン削減のために網羅的な検索より、MCP を経由する
    - グラフが生成されていない場合は、`build_or_update_graph_tool` を使用する

## Studio

- loom 用の TUI ツール
- 実装に合わせて以下を行う
    - Footer のヘルプを合わせる
    - docs/guide/studio.md の記述を合わせる
- 入力は KeyStroke で抽象化する
    - 端末差を吸収し、ロジックに集中するため
    - KeyStroke と KeyAction を紐づける

## 公開ドキュメント

- docs/ で loom の公開ドキュメントを管理する
- docs/ は VitePress によってデプロイされる
- docs/examples/live-coding/ は loom の言語仕様を網羅するように feature-*.loom ファイルを作成する
    - これは自動的にドキュメントに組み込まれる
    - テストはゴールデンテストを採用している
- README.md は外部公開向けの loom の説明に留める
    - 開発運用、ローカル開発者向け情報は AGENTS.md に書く

## ドメインドキュメント

- ドメイン用語は CONTEXT.md で管理する
    - 用語を追加・変更した場合は、公開ドキュメント内の表記も揃える
    - CONTEXT.md は glossary として扱い、実装詳細や仕様メモを書かない
- 重要な設計判断は docs/adr/ に ADR として記録する
    - 後から変更しづらい
    - 背景なしでは意図が伝わりにくい
    - 実際のトレードオフがある
  これらを満たす場合だけ ADR を追加する

## 備考

- memo*.md は実装のための個人的なメモで、必要に応じて閲覧、編集して良い
- 正式な仕様の前に、*-draft.md で切っておく
    - 仕様が採用された時は、正式なドキュメントに昇格し、*-draft.md は削除する
- DSL の実装が変更されたときや、不備が見つかった場合は、 docs/llms.md を変更すること
