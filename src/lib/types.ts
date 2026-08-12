export interface Rule {
  id: number;
  contains: string;
  contains_not: string | null;
  target_folder: string;
  enabled: boolean;
  stop_on_match: boolean;
}

export interface FileOperation {
  original_path: string;
  new_path: string;
  copied: boolean;
}

export interface SortResult {
  operations: FileOperation[];
  errors: string[];
}

/** One file's resolved destination, computed without touching the filesystem. */
export interface PreviewEntry {
  original_path: string;
  new_path: string;
  matched_rule_ids: number[];
}
