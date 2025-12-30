import { createBrowserRouter, RouterProvider } from "react-router-dom";

import Layout from "./components/layout/Layout";
import Dashboard from "./pages/Dashboard";
import Accounts from "./pages/Accounts";
import Settings from "./pages/Settings";
import ApiProxy from "./pages/ApiProxy";
import Logs from "./pages/Logs";
import ThemeManager from "./components/common/ThemeManager";
import { useEffect } from "react";
import { useConfigStore } from "./stores/useConfigStore";
import { useTranslation } from "react-i18next";

const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    children: [
      {
        index: true,
        element: <Dashboard />,
      },
      {
        path: "accounts",
        element: <Accounts />,
      },
      {
        path: "proxy",
        element: <ApiProxy />,
      },
      {
        path: "logs",
        element: <Logs />,
      },
      {
        path: "settings",
        element: <Settings />,
      },
    ],
  },
]);

function App() {
  const { config, loadConfig } = useConfigStore();
  const { i18n } = useTranslation();

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Sync language from config
  useEffect(() => {
    if (config?.language) {
      i18n.changeLanguage(config.language);
    }
  }, [config?.language, i18n]);

  return (
    <>
      <ThemeManager />
      <RouterProvider router={router} />
    </>
  );
}

export default App;
