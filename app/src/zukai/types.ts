// 図解データ v1 の型。Rust 側（crates/mondeep-core/src/diagram.rs）と同じ形。
export type Side = "self" | "external" | "error";

export interface ConceptNode {
  id: string;
  label: string;
  role?: string;
  side?: Side;
  row: number;
  col: number;
  x?: number;
  y?: number;
  highlight?: boolean;
}
export interface Edge {
  from: string;
  to: string;
  label: string;
  dashed?: boolean;
  highlight?: boolean;
}
export interface Group {
  id: string;
  label: string;
  members: string[];
  side?: Side;
}
export interface ConceptDiagram {
  version: 1;
  kind: "concept";
  title?: string;
  nodes: ConceptNode[];
  edges?: Edge[];
  groups?: Group[];
}

export interface Participant {
  id: string;
  label: string;
  side?: Side;
}
export interface Message {
  from: string;
  to: string;
  label: string;
  style?: "sync" | "async" | "reply";
  highlight?: boolean;
}
export interface Fragment {
  kind: "alt" | "opt" | "loop";
  label: string;
  start: number;
  end: number;
  else_start?: number;
  else_label?: string;
}
export interface SequenceDiagram {
  version: 1;
  kind: "sequence";
  title?: string;
  participants: Participant[];
  messages: Message[];
  fragment?: Fragment;
}

export interface StateNode {
  id: string;
  label: string;
  row: number;
  col: number;
  x?: number;
  y?: number;
  initial?: boolean;
  final?: boolean;
  side?: Side;
  highlight?: boolean;
}
export interface Transition {
  from: string;
  to: string;
  event: string;
  guard?: string;
  action?: string;
  highlight?: boolean;
}
export interface StateDiagram {
  version: 1;
  kind: "state";
  title?: string;
  states: StateNode[];
  transitions: Transition[];
}

export interface ComparisonDiagram {
  version: 1;
  kind: "comparison";
  title?: string;
  columns: string[];
  rows: { label: string; cells: string[] }[];
  highlights?: { row: number; col: number }[];
}

export type Diagram = ConceptDiagram | SequenceDiagram | StateDiagram | ComparisonDiagram;
