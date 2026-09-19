// Router minimal — ganti GoRouter (spec §16).
// SvelteKit file-routing nyusul Fase 4.
export type Route = "splash" | "main";

class RouteStore {
  current = $state<Route>("splash");

  go(route: Route) {
    this.current = route;
  }
}

export const routeStore = new RouteStore();
