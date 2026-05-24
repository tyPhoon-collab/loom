use crate::compiler::error::{CompileError, CompileResult, InvalidModifierStructureData};
use crate::dsl::token::{ModifierKind, ModifierValue, Token};

/// Resolved modifier values for a line, flattened per block (DFS leaf order)
#[derive(Debug, Clone)]
pub enum ResolvedModifierValue {
    Scalar(i32),
    PerNote(Vec<i32>),
}

pub struct ResolvedModifiers {
    pub velocities: Vec<Vec<ResolvedModifierValue>>, // per block, per leaf token (DFS order)
    pub pitches: Vec<Vec<ResolvedModifierValue>>,    // per block, per leaf token (DFS order)
}

/// Count the number of leaf tokens (non-Group) in a token tree via DFS
pub fn count_leaf_tokens(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .map(|t| match t {
            Token::Group(sub) => count_leaf_tokens(sub),
            _ => 1,
        })
        .sum()
}

/// Expand modifier values to align with pattern leaf tokens.
///
/// - `ModifierValue::Group(vals)` aligns its sub-values with the sub-tokens of the
///   corresponding `Token::Group`.
/// - A scalar modifier value at a Group token position is broadcast to all
///   leaf tokens of that group.
/// - `ModifierValue::Empty` at a Group position fills all leaves with Empty.
pub fn expand_modifier_values(
    tokens: &[Token],
    mod_values: &[ModifierValue],
    track_name: &str,
    context: &str,
    modifier_kind: ModifierKind,
    block_index: usize,
    path: &mut Vec<usize>,
) -> CompileResult<Vec<ModifierValue>> {
    let mut result = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        path.push(i);
        let mod_val = mod_values.get(i);
        match token {
            Token::Group(sub_tokens) => {
                let leaf_count = count_leaf_tokens(sub_tokens);
                match mod_val {
                    Some(ModifierValue::Group(sub_vals)) => {
                        // Recurse: align sub-values with sub-tokens
                        result.extend(expand_modifier_values(
                            sub_tokens,
                            sub_vals,
                            track_name,
                            context,
                            modifier_kind,
                            block_index,
                            path,
                        )?);
                    }
                    Some(val @ (ModifierValue::Set(_) | ModifierValue::Latch(_))) => {
                        // Broadcast scalar to all leaves in the group
                        for _ in 0..leaf_count {
                            result.push(val.clone());
                        }
                    }
                    Some(ModifierValue::NoteList(_)) => {
                        let value_path = path
                            .iter()
                            .map(|idx| idx.to_string())
                            .collect::<Vec<_>>()
                            .join(".");
                        return Err(CompileError::InvalidModifierStructure(Box::new(
                            InvalidModifierStructureData {
                                track: track_name.to_string(),
                                context: context.to_string(),
                                modifier: modifier_kind.to_string(),
                                block_index,
                                value_path,
                                reason: "note-list value cannot be applied to a group token"
                                    .to_string(),
                            },
                        )));
                    }
                    Some(ModifierValue::Empty) | None => {
                        // Fill with Empty for all leaves
                        for _ in 0..leaf_count {
                            result.push(ModifierValue::Empty);
                        }
                    }
                }
            }
            _ => {
                // Leaf token: use modifier value directly, or Empty
                match mod_val {
                    Some(ModifierValue::Group(_)) => {
                        let value_path = path
                            .iter()
                            .map(|idx| idx.to_string())
                            .collect::<Vec<_>>()
                            .join(".");
                        return Err(CompileError::InvalidModifierStructure(Box::new(
                            InvalidModifierStructureData {
                                track: track_name.to_string(),
                                context: context.to_string(),
                                modifier: modifier_kind.to_string(),
                                block_index,
                                value_path,
                                reason: "group value cannot be applied to a non-group token"
                                    .to_string(),
                            },
                        )));
                    }
                    Some(val) => result.push(val.clone()),
                    None => result.push(ModifierValue::Empty),
                }
            }
        }
        path.pop();
    }

    Ok(result)
}
