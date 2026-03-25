export interface Rule {
  id: number;
  contains: string;
  contains_not: string | null;
  target_folder: string;
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
