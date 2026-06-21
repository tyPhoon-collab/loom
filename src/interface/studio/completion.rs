use super::keystroke::{key_stroke_matches, KeyStroke};
use super::source::parse_source_frontmatter;
use super::source_text::char_to_byte_index;
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::{IntoDiagnostic, Result};
use ratatui_textarea::CursorMove;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_COMPLETION_ITEMS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CompletionState {
    trigger: CompletionTrigger,
    anchor: (usize, usize),
    query: String,
    candidates: Vec<CompletionCandidate>,
    selected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionTrigger {
    TemplateCall,
    FragmentCall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CompletionCandidate {
    pub(super) name: String,
    pub(super) insert_text: String,
    pub(super) label: &'static str,
    source_order: CandidateSourceOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CandidateSourceOrder {
    LocalTemplate,
    LibraryTemplate,
    Fragment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionKey {
    Accept,
    Close,
    Next,
    Previous,
}

impl CompletionState {
    pub(super) fn title(&self) -> &'static str {
        match self.trigger {
            CompletionTrigger::TemplateCall => " Template ",
            CompletionTrigger::FragmentCall => " Fragment ",
        }
    }

    pub(super) fn query(&self) -> &str {
        &self.query
    }

    pub(super) fn visible_candidates(&self) -> &[CompletionCandidate] {
        let end = self.candidates.len().min(MAX_COMPLETION_ITEMS);
        &self.candidates[..end]
    }

    pub(super) fn selected_index(&self) -> usize {
        self.selected
    }
}

impl StudioApp {
    pub(super) fn handle_completion_key(&mut self, key: &KeyEvent) -> Result<bool> {
        let Some(action) = completion_key(key) else {
            return Ok(false);
        };
        if self.completion.is_none() {
            return Ok(false);
        }

        match action {
            CompletionKey::Accept => self.accept_completion()?,
            CompletionKey::Close => {
                self.completion = None;
                self.status_message = "Completion closed".into();
            }
            CompletionKey::Next => self.move_completion_selection(1),
            CompletionKey::Previous => self.move_completion_selection(-1),
        }
        Ok(true)
    }

    pub(super) fn refresh_completion_after_text_input(&mut self) {
        let cursor = self.textarea.cursor();
        let Some(line) = self.textarea.lines().get(cursor.0) else {
            self.completion = None;
            return;
        };
        let Some(trigger) = completion_trigger_before_cursor(line, cursor.1) else {
            self.completion = None;
            return;
        };

        let candidates = self
            .collect_completion_candidates(trigger.query, trigger.kind)
            .unwrap_or_default();
        if candidates.is_empty() {
            self.completion = None;
            self.status_message = "No completion candidates".into();
            return;
        }

        self.completion = Some(CompletionState {
            trigger: trigger.kind,
            anchor: (cursor.0, trigger.anchor_col),
            query: trigger.query.to_string(),
            candidates,
            selected: 0,
        });
    }

    fn move_completion_selection(&mut self, delta: i32) {
        let Some(completion) = self.completion.as_mut() else {
            return;
        };
        let len = completion.visible_candidates().len();
        if len == 0 {
            return;
        }
        completion.selected = wrap_index(completion.selected, delta, len);
    }

    fn accept_completion(&mut self) -> Result<()> {
        let Some(completion) = self.completion.clone() else {
            return Ok(());
        };
        let Some(candidate) = completion
            .visible_candidates()
            .get(completion.selected)
            .cloned()
        else {
            self.completion = None;
            return Ok(());
        };

        let cursor = self.textarea.cursor();
        if cursor.0 != completion.anchor.0 || cursor.1 < completion.anchor.1 {
            self.completion = None;
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.completion = None;
            return Ok(());
        };
        let start = char_to_byte_index(line, completion.anchor.1);
        let end = char_to_byte_index(line, cursor.1);
        line.replace_range(start..end, &candidate.insert_text);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        let new_col = completion.anchor.1 + candidate.insert_text.chars().count();
        self.textarea
            .move_cursor(CursorMove::Jump(cursor.0 as u16, new_col as u16));
        self.dirty = true;
        self.completion = None;
        self.status_message = format!("Completed {}", candidate.insert_text);
        Ok(())
    }

    fn collect_completion_candidates(
        &self,
        query: &str,
        trigger: CompletionTrigger,
    ) -> Result<Vec<CompletionCandidate>> {
        let candidates = match trigger {
            CompletionTrigger::TemplateCall => self.template_completion_candidates()?,
            CompletionTrigger::FragmentCall => self.fragment_completion_candidates()?,
        };
        Ok(rank_completion_candidates(query, candidates))
    }

    fn template_completion_candidates(&self) -> Result<Vec<CompletionCandidate>> {
        let mut candidates = Vec::new();
        let source = self.source();
        let mut seen = HashSet::new();

        for name in template_headers(&source) {
            if seen.insert(format!("local:{name}")) {
                candidates.push(CompletionCandidate {
                    insert_text: format!("[@{}]", name),
                    name,
                    label: "local template",
                    source_order: CandidateSourceOrder::LocalTemplate,
                });
            }
        }

        let Some((library_source, library_path)) = self.template_library_mapping_source()? else {
            return Ok(candidates);
        };
        let Some(frontmatter) = parse_source_frontmatter(&library_source)? else {
            return Ok(candidates);
        };
        let base_dir = library_path.parent().unwrap_or_else(|| Path::new("."));
        let mut aliases: Vec<_> = frontmatter.templates.into_iter().collect();
        aliases.sort_by(|a, b| a.0.cmp(&b.0));
        for (alias, mapped) in aliases {
            let Some(path) = resolve_mapped_path(base_dir, &mapped) else {
                continue;
            };
            let Ok(source) = fs::read_to_string(path) else {
                continue;
            };
            for name in template_headers(&source) {
                let display = format!("{}.{}", alias, name);
                if seen.insert(format!("library:{display}")) {
                    candidates.push(CompletionCandidate {
                        insert_text: format!("[@{}]", display),
                        name: display,
                        label: "library template",
                        source_order: CandidateSourceOrder::LibraryTemplate,
                    });
                }
            }
        }
        Ok(candidates)
    }

    fn template_library_mapping_source(&self) -> Result<Option<(String, PathBuf)>> {
        if let Some(manifest_path) = &self.manifest_path {
            if manifest_path != &self.path {
                let source = fs::read_to_string(manifest_path).into_diagnostic()?;
                return Ok(Some((source, manifest_path.clone())));
            }
        }
        Ok(Some((self.source(), self.path.clone())))
    }

    fn fragment_completion_candidates(&self) -> Result<Vec<CompletionCandidate>> {
        if self
            .manifest_path
            .as_ref()
            .is_some_and(|manifest_path| manifest_path != &self.path)
        {
            return Ok(Vec::new());
        }
        let Some(frontmatter) = parse_source_frontmatter(&self.source())? else {
            return Ok(Vec::new());
        };
        let mut names: Vec<_> = frontmatter.fragments.into_keys().collect();
        names.sort();
        Ok(names
            .into_iter()
            .map(|name| CompletionCandidate {
                insert_text: format!("[[{}]]", name),
                name,
                label: "fragment",
                source_order: CandidateSourceOrder::Fragment,
            })
            .collect())
    }
}

struct ActiveTrigger<'a> {
    kind: CompletionTrigger,
    anchor_col: usize,
    query: &'a str,
}

fn completion_key(key: &KeyEvent) -> Option<CompletionKey> {
    if key_stroke_matches(KeyStroke::Code(KeyCode::Enter), key) {
        Some(CompletionKey::Accept)
    } else if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), key) {
        Some(CompletionKey::Close)
    } else if key_stroke_matches(KeyStroke::CtrlChar('n'), key)
        || key_stroke_matches(KeyStroke::Code(KeyCode::Down), key)
    {
        Some(CompletionKey::Next)
    } else if key_stroke_matches(KeyStroke::CtrlChar('p'), key)
        || key_stroke_matches(KeyStroke::Code(KeyCode::Up), key)
    {
        Some(CompletionKey::Previous)
    } else {
        None
    }
}

fn completion_trigger_before_cursor(line: &str, cursor_col: usize) -> Option<ActiveTrigger<'_>> {
    let prefix_end = char_to_byte_index(line, cursor_col);
    let prefix = &line[..prefix_end];
    let template = prefix.rfind("[@").map(|index| {
        let query = &prefix[index + 2..];
        (CompletionTrigger::TemplateCall, index, query)
    });
    let fragment = prefix.rfind("[[").map(|index| {
        let query = &prefix[index + 2..];
        (CompletionTrigger::FragmentCall, index, query)
    });
    let (kind, anchor_col, query) = match (template, fragment) {
        (Some(t), Some(f)) if t.1 > f.1 => t,
        (Some(t), Some(_)) => t,
        (Some(t), None) => t,
        (None, Some(f)) => f,
        (None, None) => return None,
    };
    if !is_valid_completion_query(query) {
        return None;
    }
    Some(ActiveTrigger {
        kind,
        anchor_col: prefix[..anchor_col].chars().count(),
        query,
    })
}

fn is_valid_completion_query(query: &str) -> bool {
    query
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn template_headers(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("# @")
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
        })
        .collect()
}

fn resolve_mapped_path(base_dir: &Path, mapped: &str) -> Option<PathBuf> {
    let mapped_path = Path::new(mapped);
    if mapped_path.is_absolute()
        || mapped_path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return None;
    }
    Some(base_dir.join(mapped_path))
}

fn rank_completion_candidates(
    query: &str,
    candidates: Vec<CompletionCandidate>,
) -> Vec<CompletionCandidate> {
    let mut scored: Vec<_> = candidates
        .into_iter()
        .filter_map(|candidate| fuzzy_score(query, &candidate.name).map(|score| (score, candidate)))
        .collect();
    scored.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.source_order.cmp(&right.source_order))
            .then_with(|| left.name.cmp(&right.name))
    });
    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query = query.to_ascii_lowercase();
    let candidate = candidate.to_ascii_lowercase();
    if candidate.starts_with(&query) {
        return Some(10_000 - candidate.len() as i32);
    }

    let mut score = 0;
    let mut last_match: Option<usize> = None;
    let mut chars = candidate.char_indices();
    for q in query.chars() {
        let (idx, _) = chars.find(|(_, ch)| *ch == q)?;
        score += 10;
        if last_match.is_some_and(|prev| prev + 1 == idx) {
            score += 5;
        }
        if idx == 0 || matches!(candidate.as_bytes().get(idx - 1), Some(b'.' | b'-' | b'_')) {
            score += 3;
        }
        last_match = Some(idx);
    }

    Some(score - candidate.len() as i32)
}

fn wrap_index(index: usize, delta: i32, len: usize) -> usize {
    if delta < 0 {
        index.checked_sub(1).unwrap_or(len - 1)
    } else {
        (index + 1) % len
    }
}

#[cfg(test)]
mod tests {
    use super::{
        completion_trigger_before_cursor, fuzzy_score, rank_completion_candidates,
        CandidateSourceOrder, CompletionCandidate, CompletionTrigger,
    };

    fn candidate(name: &str, source_order: CandidateSourceOrder) -> CompletionCandidate {
        CompletionCandidate {
            name: name.to_string(),
            insert_text: format!("[@{}]", name),
            label: "test",
            source_order,
        }
    }

    #[test]
    fn fuzzy_score_matches_subsequence() {
        assert!(fuzzy_score("k4b", "kick.4beat").is_some());
        assert!(fuzzy_score("kb4", "kick.4beat").is_none());
    }

    #[test]
    fn rank_prefers_prefix_then_local_templates() {
        let ranked = rank_completion_candidates(
            "ki",
            vec![
                candidate("drums.kick", CandidateSourceOrder::LibraryTemplate),
                candidate("kick", CandidateSourceOrder::LocalTemplate),
            ],
        );

        assert_eq!(ranked[0].name, "kick");
    }

    #[test]
    fn trigger_reads_template_query() {
        let trigger = completion_trigger_before_cursor("[@k4", 4).unwrap();

        assert_eq!(trigger.kind, CompletionTrigger::TemplateCall);
        assert_eq!(trigger.anchor_col, 0);
        assert_eq!(trigger.query, "k4");
    }

    #[test]
    fn trigger_reads_fragment_query() {
        let trigger = completion_trigger_before_cursor("  [[ver", 7).unwrap();

        assert_eq!(trigger.kind, CompletionTrigger::FragmentCall);
        assert_eq!(trigger.anchor_col, 2);
        assert_eq!(trigger.query, "ver");
    }

    #[test]
    fn trigger_rejects_closed_call_prefix() {
        assert!(completion_trigger_before_cursor("[@kick]", 7).is_none());
    }
}
