import { useCallback, useEffect, useState } from "preact/hooks";
import { playgroundEffects } from "../effects/effects";
import type { Command, Dispatch, Effects, Message, Model, UpdateResult } from "../domain/model";
import { update } from "../domain/model";

export function useElmish(initial: UpdateResult): readonly [Model, Dispatch] {
  const [snapshot, setSnapshot] = useState<{ model: Model; commands: readonly Command[] }>(() => ({
    model: initial[0],
    commands: initial[1],
  }));

  const dispatch = useCallback((message: Message) => {
    setSnapshot((current) => {
      const [model, commands] = update(current.model, message);
      return { model, commands };
    });
  }, []);

  useEffect(() => {
    for (const command of snapshot.commands) {
      runCommand(command, playgroundEffects, dispatch);
    }
  }, [snapshot.commands, dispatch]);

  return [snapshot.model, dispatch];
}

function runCommand(command: Command, effects: Effects, dispatch: Dispatch): void {
  void command(effects, dispatch);
}
