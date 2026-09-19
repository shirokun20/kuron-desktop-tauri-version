// TS mirror entitas Rust (spec §11). Nanti generate via ts-rs (ADR-004);
// tulis manual minimal selama Fase 0.

export interface Hello {
  name: string;
  message: string;
}

export interface AppInfo {
  name: string;
  version: string;
  backend: string;
}

export interface Content {
  id: string;
  title: string;
  cover_url: string;
  source_id: string;
  upload_date: string | null;
  is_favorite: boolean;
}
