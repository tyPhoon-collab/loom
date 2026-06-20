use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub(super) struct ActivePreviewNote {
    pub(super) channel: u8,
    pub(super) note: u8,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PreviewPanelState {
    pub(super) open: bool,
    pub(super) track_header_row: Option<usize>,
    pub(super) track_name: String,
    pub(super) channel: u8,
    pub(super) selected_target: PreviewTarget,
    pub(super) source_program: Option<u8>,
    pub(super) override_program: Option<u8>,
    pub(super) controls: PreviewControls,
    pub(super) velocity: u8,
    pub(super) active_keys: HashSet<char>,
}

impl PreviewPanelState {
    pub(super) fn effective_program(&self) -> Option<u8> {
        self.override_program.or(self.source_program)
    }

    pub(super) fn effective_control_value(&self, target: PreviewTarget) -> Option<u8> {
        let spec = target.control_spec()?;
        self.controls
            .get(target)
            .map(|state| state.effective_value(spec))
    }

    pub(super) fn reset_overrides(&mut self) {
        self.override_program = None;
        self.controls.clear_overrides();
    }

    pub(super) fn has_overrides(&self) -> bool {
        self.override_program.is_some() || self.controls.has_overrides()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PreviewTarget {
    #[default]
    Program,
    Volume,
    Pan,
    Expression,
    Mod,
}

impl PreviewTarget {
    pub(super) const ALL: [Self; 5] = [
        Self::Program,
        Self::Volume,
        Self::Pan,
        Self::Expression,
        Self::Mod,
    ];

    pub(super) fn from_index(index: char) -> Option<Self> {
        match index {
            '1' => Some(Self::Program),
            '2' => Some(Self::Volume),
            '3' => Some(Self::Pan),
            '4' => Some(Self::Expression),
            '5' => Some(Self::Mod),
            _ => None,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Program => "PC",
            Self::Volume => "VOL",
            Self::Pan => "PAN",
            Self::Expression => "EXP",
            Self::Mod => "MOD",
        }
    }

    pub(super) fn control_spec(self) -> Option<PreviewControlSpec> {
        match self {
            Self::Program => None,
            Self::Volume => Some(PreviewControlSpec {
                target: self,
                cc: 7,
                default_value: 100,
                canonical_label: "volume",
            }),
            Self::Pan => Some(PreviewControlSpec {
                target: self,
                cc: 10,
                default_value: 64,
                canonical_label: "pan",
            }),
            Self::Expression => Some(PreviewControlSpec {
                target: self,
                cc: 11,
                default_value: 100,
                canonical_label: "expression",
            }),
            Self::Mod => Some(PreviewControlSpec {
                target: self,
                cc: 1,
                default_value: 0,
                canonical_label: "mod",
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PreviewControlSpec {
    pub(super) target: PreviewTarget,
    pub(super) cc: u8,
    pub(super) default_value: u8,
    pub(super) canonical_label: &'static str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PreviewControlState {
    pub(super) source: Option<u8>,
    pub(super) override_value: Option<u8>,
}

impl PreviewControlState {
    pub(super) fn effective_value(self, spec: PreviewControlSpec) -> u8 {
        self.override_value
            .or(self.source)
            .unwrap_or(spec.default_value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct PreviewControls {
    pub(super) volume: PreviewControlState,
    pub(super) pan: PreviewControlState,
    pub(super) expression: PreviewControlState,
    pub(super) mod_wheel: PreviewControlState,
}

impl PreviewControls {
    pub(super) fn get(&self, target: PreviewTarget) -> Option<PreviewControlState> {
        match target {
            PreviewTarget::Program => None,
            PreviewTarget::Volume => Some(self.volume),
            PreviewTarget::Pan => Some(self.pan),
            PreviewTarget::Expression => Some(self.expression),
            PreviewTarget::Mod => Some(self.mod_wheel),
        }
    }

    pub(super) fn get_mut(&mut self, target: PreviewTarget) -> Option<&mut PreviewControlState> {
        match target {
            PreviewTarget::Program => None,
            PreviewTarget::Volume => Some(&mut self.volume),
            PreviewTarget::Pan => Some(&mut self.pan),
            PreviewTarget::Expression => Some(&mut self.expression),
            PreviewTarget::Mod => Some(&mut self.mod_wheel),
        }
    }

    pub(super) fn clear_overrides(&mut self) {
        self.volume.override_value = None;
        self.pan.override_value = None;
        self.expression.override_value = None;
        self.mod_wheel.override_value = None;
    }

    pub(super) fn has_overrides(&self) -> bool {
        self.volume.override_value.is_some()
            || self.pan.override_value.is_some()
            || self.expression.override_value.is_some()
            || self.mod_wheel.override_value.is_some()
    }
}
