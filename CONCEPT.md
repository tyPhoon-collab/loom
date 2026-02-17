# Loom

テキストでMIDIを織りなす、エンジニアのための音楽専用織り機

## 1. コンセプト

- Markdown風: 見出しでトラック管理、YAML Frontmatterでメタデータ。
- 疎なピアノロール (Sparse Piano Roll): 必要な音階行のみ記述。
- Elastic Grid: `|` で区切られた時間を等分割（Tidal方式）。スペースで見た目を整形しても演奏には影響しない。
- Nested Cycle: `[]` を使うことで、特定の一拍をさらに等分割（再帰的分割）可能。
- Stateful Sustain: ブロック（小節）をまたいで音を継続（タイ）可能。

## 2. 構文ルール (Syntax)

| 記号 | 名前 | 機能・挙動 |
| :--- | :--- | :--- |
| `---` | Frontmatter | ファイル先頭でBPM、拍子、単位時間を定義。 |
| `#` | Track Header | トラック名とMIDIチャンネルを定義。例: `# Piano: 1` |
| `>` | Comment | 人間用のコメント。 |
| `|` | Bar/Beat | 1小節（または指定単位）を区切る境界線。 |
| `^` | Note On | 発音（Trigger）。 |
| `-` | Sustain | 前の音を伸ばす。行頭にある場合は前のブロックから継続（タイ）する。 |
| `.` | Rest | 休符（明示的な無音）。 |
| `[ ]` | Nest | ネスト。囲まれた範囲を「1単位」としてさらに分割する。 |
| ` ` | Padding | 無視される。フォーマッタが縦を揃えるために挿入する。 |
| `Note` | Row Header | 音階名。例: `c3`, `kick`。 |

### Frontmatter (YAML)

| キー | 必須 | 型 | デフォルト | 説明 |
| :--- | :--- | :--- | :--- | :--- |
| `bpm` | Yes | Int | - | 曲のテンポ（Beats Per Minute）。 |
| `signature` | No | String | `4/4` | 拍子。`unit: bar` の際の時間計算基準となる。 |
| `unit` | No | Enum | `bar` | `bar` (小節) または `beat` (拍)。 |
| `title` | No | String | - | 曲名。 |
| `author` | No | String | - | 作者名。 |

### Note (Row Header)

- フォーマット: `[NoteName][Octave]` (例: `c3`, `g#2`, `ab4`)
- ドラム: `kick`, `snare`, `hi-hat`, etc... (General MIDIマッピング準拠予定)

## 3. サンプルコード (v0.3)

```markdown
---
bpm: 120
signature: 4/4
unit: bar
---

# Piano: 1

> 基本: |...| を「1小節」とみなす (unit: bar, signature: 4/4 なので4拍)
> 上の行は4分音符、下の行は3連符（ポリリズム）
> c3の行末から次行頭へのタイ（--- | -..）に注目

g3| ^ . ^ . |       | ^ . . |
e3| ^ ^ ^   |       | . ^ . |
c3| ^       | ^ - - | - . ^ |

# Drums: 10

> Kickは4つ打ち
> Snareの2拍目に注目: [^ ^ ^] で1拍の中に3連打（ロール）を入れる

kick  | ^ .  ^      . ^ . ^ . |
snare | . . [^ ^ ^] . . . ^ . |
hi-hat| . ^  .      ^ . ^ . ^ |
```

## 4. 実装仕様 (Logic)

パーサは「Elastic Grid」「Nested Cycle」に加え、「Stateful Sustain」を以下のロジックで解釈します。

### A. トークン化と時間計算

1. 行の分割: `|` で文字列を分割し、ブロックを取り出す。
2. トークン化:
   - スペースを除去する（パディング無視）。
   - `[` と `]` を認識し、ネストされたグループを「1つのトークン」として扱う。
   - 例: `^ . [^ ^ ^] .` → 4つのトークン `^`, `.`, `[^ ^ ^]`, `.` として扱う。
3. 再帰的イベント計算:
   - ブロック全体の時間（`unit: bar` なら4拍）をトークン数で割る。
   - ネストがあれば再帰的に分割する。

### B. ステート管理（タイの実装）

行（Row）ごとに「最後のイベント状態（Last Event State）」を保持します。

1. 行頭チェック:
   - ブロックの最初のトークンが `-` (Sustain) である場合：
     - 「直前のブロックの最後のNote Onイベント」を検索する。
     - そのNote Onイベントの `Duration` を、現在の `-` の分だけ加算（延長）する。
     - 新しいNote Onイベントは生成しない。
2. 行途中:
   - `^` (Note On) が来たら新規イベント生成。状態を「On」にする。
   - `-` (Sustain) が来たら直前のNote Onの `Duration` を加算。
   - `.` (Rest) が来たら、状態を「Off」にする（厳密にはNote Offイベントを発行するというよりは、Sustainの対象外になる）。

**例:**

```text
Block 1: | ^ - |  (2分音符)
Block 2: | - . |  (4分音符分のタイ + 休符)
```

1. Block 1解析: `^` 生成。Duration = 2拍 (この時点)。State = On(EventID: X)。
2. Block 2解析: 先頭が `-`。前のStateがOn(EventID: X)なので、EventID: X のDurationに1拍加算。Total Duration = 3拍。
3. Block 2続き: 次が `.`。ここでState = Off。
