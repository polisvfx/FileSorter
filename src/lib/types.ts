/** Which part of the file the Contains expression is tested against. */
export type MatchScope = 'name' | 'stem' | 'extension' | 'path';

export interface Rule {
  id: number;
  contains: string;
  contains_not: string | null;
  target_folder: string;
  enabled: boolean;
  stop_on_match: boolean;
  /** Treat Contains / Contains NOT as regex; the `,` and `*` operators don't apply. */
  regex: boolean;
  case_sensitive: boolean;
  scope: MatchScope;
}

export interface FileOperation {
  original_path: string;
  new_path: string;
  copied: boolean;
}

export interface SortResult {
  operations: FileOperation[];
  errors: string[];
  /** True when the user stopped the run early; moved files are still undoable. */
  cancelled: boolean;
}

/** A dropped file or folder, kept unexpanded. */
export interface PathInfo {
  path: string;
  is_dir: boolean;
  file_count: number;
}

/** One file's resolved destination, computed without moving anything. */
export interface PreviewEntry {
  original_path: string;
  new_path: string;
  matched_rule_ids: number[];
}

/** A rule whose pattern would not compile — surfaced on the rule itself. */
export interface RuleError {
  rule_id: number;
  message: string;
}

export interface PreviewResult {
  entries: PreviewEntry[];
  rule_errors: RuleError[];
}
