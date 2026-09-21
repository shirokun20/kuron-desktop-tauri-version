// Router minimal — ganti GoRouter (spec §16).
// SvelteKit file-routing nyusul Fase 4.
// Detail & reader = route khusus fullscreen (tanpa sidebar) ala mobile
// `DetailScreen` / reader push — bukan panel di dalam MainPage.
import type { Chapter, Content } from "../domain/types";

export type Route = "splash" | "main" | "detail" | "reader";

class RouteStore {
  current = $state<Route>("splash");
  detailContent = $state<Content | null>(null);
  readerChapter = $state<Chapter | null>(null);
  readerList = $state<Chapter[]>([]);

  go(route: Route) {
    this.current = route;
  }

  openDetail(c: Content) {
    this.detailContent = c;
    this.readerChapter = null;
    this.readerList = [];
    this.current = "detail";
  }

  openReader(ch: Chapter, list: Chapter[]) {
    this.readerChapter = ch;
    this.readerList = list;
    this.current = "reader";
  }

  backToDetail() {
    this.readerChapter = null;
    this.current = "detail";
  }

  backToMain() {
    this.detailContent = null;
    this.readerChapter = null;
    this.readerList = [];
    this.current = "main";
  }
}

export const routeStore = new RouteStore();
