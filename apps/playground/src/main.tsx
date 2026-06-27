import "@knadh/oat/oat.min.css";
import { render } from "preact";
import { useMemo } from "preact/hooks";
import { App } from "./app/App";
import { initModel } from "./domain/model";
import { useElmish } from "./app/runtime";
import "./styles.css";

const appRoot = document.querySelector<HTMLDivElement>("#app");

if (!appRoot) {
  throw new Error("Missing #app root");
}

function Root() {
  const initial = useMemo(() => initModel(window.location), []);
  const [model, dispatch] = useElmish(initial);
  return <App model={model} dispatch={dispatch} />;
}

render(<Root />, appRoot);
