export type MidiEvent =
  | {
      Note: NoteEvent;
    }
  | {
      ControlChange: {
        time: number;
        channel: number;
        cc: number;
        value: number;
      };
    }
  | {
      ProgramChange: {
        time: number;
        channel: number;
        program: number;
      };
    };

export type NoteEvent = {
  time: number;
  duration: number;
  channel: number;
  note: number;
  velocity: number;
};
