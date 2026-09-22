import { createRootRoute, createRoute, createRouter, redirect } from "@tanstack/react-router";

import { AskPage } from "./pages/AskPage";
import { DashboardPage } from "./pages/DashboardPage";
import { FilesPage } from "./pages/FilesPage";
import { GraphPage } from "./pages/GraphPage";
import { InsightsPage } from "./pages/InsightsPage";
import { OverviewPage } from "./pages/OverviewPage";
import { RepositoriesPage } from "./pages/RepositoriesPage";
import { SearchPage } from "./pages/SearchPage";
import { AppShell } from "./shell/AppShell";

function symbolSearch(search: Record<string, unknown>) {
  return { symbol: typeof search.symbol === "string" ? search.symbol : undefined };
}

const rootRoute = createRootRoute({
  component: AppShell,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/dashboard" });
  },
});

const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/dashboard",
  component: DashboardPage,
});

const repositoriesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories",
  component: RepositoriesPage,
});

const repoRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id",
  beforeLoad: ({ params }) => {
    throw redirect({ to: "/repositories/$id/overview", params });
  },
});

const overviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/overview",
  component: OverviewPage,
});

const filesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/files",
  validateSearch: symbolSearch,
  component: FilesPage,
});

const graphRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/graph",
  validateSearch: symbolSearch,
  component: GraphPage,
});

const searchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/search",
  component: SearchPage,
});

const aiRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/ai",
  component: AskPage,
});

const insightsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/repositories/$id/insights",
  validateSearch: symbolSearch,
  component: InsightsPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  dashboardRoute,
  repositoriesRoute,
  repoRoute,
  overviewRoute,
  filesRoute,
  graphRoute,
  searchRoute,
  aiRoute,
  insightsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
