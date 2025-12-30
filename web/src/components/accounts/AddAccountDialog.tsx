import { useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { Plus, Loader2, CheckCircle2, XCircle, Database } from "lucide-react";
import { useAccountStore } from "../../stores/useAccountStore";
import { useTranslation } from "react-i18next";

interface AddAccountDialogProps {
  onAdd: (email: string, refreshToken: string) => Promise<void>;
}

type Status = "idle" | "loading" | "success" | "error";

function AddAccountDialog({ onAdd }: AddAccountDialogProps) {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"token" | "import">("import");
  const [refreshToken, setRefreshToken] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [message, setMessage] = useState("");

  const { importJsonAccounts } = useAccountStore();

  const resetState = () => {
    setStatus("idle");
    setMessage("");
    setRefreshToken("");
  };

  useEffect(() => {
    if (isOpen) resetState();
  }, [isOpen]);

  const handleSubmit = async () => {
    if (!refreshToken) {
      setStatus("error");
      setMessage(t("accounts.add.token.error_token"));
      return;
    }

    setStatus("loading");
    // Simple logic: treat as single account unless it looks like array
    try {
      await onAdd("", refreshToken);
      setStatus("success");
      setMessage(t("common.success"));
      setTimeout(() => {
        setIsOpen(false);
      }, 1500);
    } catch (e) {
      setStatus("error");
      setMessage(String(e));
    }
  };

  const handleFileUpload = async (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setStatus("loading");
    setMessage("Importing...");

    try {
      const content = await file.text();
      let jsonContent;
      try {
        jsonContent = JSON.parse(content);
      } catch (e) {
        throw new Error("Invalid JSON file");
      }

      await importJsonAccounts(jsonContent);

      setStatus("success");
      setMessage(t("common.success"));
      setTimeout(() => {
        setIsOpen(false);
        resetState();
      }, 1500);
    } catch (error) {
      setStatus("error");
      setMessage(`${t("common.error")}: ${String(error)}`);
    }

    // Reset input
    event.target.value = "";
  };

  // Status Alert Component
  const StatusAlert = () => {
    if (status === "idle" || !message) return null;
    const styles = {
      loading: "alert-info",
      success: "alert-success",
      error: "alert-error",
    };
    const icons = {
      loading: <Loader2 className="w-5 h-5 animate-spin" />,
      success: <CheckCircle2 className="w-5 h-5" />,
      error: <XCircle className="w-5 h-5" />,
    };

    return (
      <div className={`alert ${styles[status]} mb-4 text-sm py-2 shadow-sm`}>
        {icons[status]}
        <span>{message}</span>
      </div>
    );
  };

  return (
    <>
      <button
        className="px-4 py-2 bg-white dark:bg-base-100 text-gray-700 dark:text-gray-300 text-sm font-medium rounded-lg hover:bg-gray-50 dark:hover:bg-base-200 transition-colors flex items-center gap-2 shadow-sm border border-gray-200/50 dark:border-base-300"
        onClick={() => setIsOpen(true)}
      >
        <Plus className="w-4 h-4" />
        {t("accounts.add_account")}
      </button>

      {isOpen &&
        createPortal(
          <dialog className="modal modal-open z-[100]">
            <div className="modal-box bg-white dark:bg-base-100 text-gray-900 dark:text-base-content">
              <h3 className="font-bold text-lg mb-4">
                {t("accounts.add.title")}
              </h3>

              {/* Tabs */}
              <div className="bg-gray-100 dark:bg-base-200 p-1 rounded-xl mb-6 grid grid-cols-2 gap-1">
                <button
                  className={`py-2 px-3 rounded-lg text-sm font-medium transition-all duration-200 ${
                    activeTab === "import"
                      ? "bg-white dark:bg-base-100 shadow-sm text-blue-600 dark:text-blue-400"
                      : "text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
                  } `}
                  onClick={() => setActiveTab("import")}
                >
                  {t("accounts.add.tabs.import")} (JSON)
                </button>
                <button
                  className={`py-2 px-3 rounded-lg text-sm font-medium transition-all duration-200 ${
                    activeTab === "token"
                      ? "bg-white dark:bg-base-100 shadow-sm text-blue-600 dark:text-blue-400"
                      : "text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
                  } `}
                  onClick={() => setActiveTab("token")}
                >
                  Refresh Token
                </button>
              </div>

              <StatusAlert />

              <div className="min-h-[150px]">
                {activeTab === "import" && (
                  <div className="space-y-6 py-2">
                    <div
                      className="text-center p-8 border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-xl hover:border-blue-500 transition-colors cursor-pointer bg-gray-50 dark:bg-base-200"
                      onClick={() =>
                        document.getElementById("json-file-input")?.click()
                      }
                    >
                      <Database className="w-10 h-10 text-gray-400 mx-auto mb-2" />
                      <p className="text-sm text-gray-500">
                        Click to upload JSON file
                      </p>
                      <input
                        type="file"
                        id="json-file-input"
                        className="hidden"
                        accept=".json"
                        onChange={handleFileUpload}
                      />
                    </div>
                  </div>
                )}

                {activeTab === "token" && (
                  <div className="space-y-4 py-2">
                    <textarea
                      className="textarea textarea-bordered w-full h-32 font-mono text-xs"
                      placeholder="Input Refresh Token"
                      value={refreshToken}
                      onChange={(e) => setRefreshToken(e.target.value)}
                      disabled={status === "loading" || status === "success"}
                    />
                  </div>
                )}
              </div>

              <div className="flex gap-3 w-full mt-6">
                <button
                  className="flex-1 px-4 py-2.5 bg-gray-100 dark:bg-base-200 text-gray-700 dark:text-gray-300 font-medium rounded-xl hover:bg-gray-200 dark:hover:bg-base-300 transition-colors"
                  onClick={() => setIsOpen(false)}
                >
                  {t("accounts.add.btn_cancel")}
                </button>
                {activeTab === "token" && (
                  <button
                    className="flex-1 px-4 py-2.5 bg-blue-500 text-white font-medium rounded-xl hover:bg-blue-600 transition-colors flex justify-center items-center gap-2"
                    onClick={handleSubmit}
                    disabled={status === "loading" || status === "success"}
                  >
                    {status === "loading" && (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    )}
                    {t("accounts.add.btn_confirm")}
                  </button>
                )}
              </div>
            </div>
            <div
              className="modal-backdrop bg-black/40 backdrop-blur-sm fixed inset-0 z-[-1]"
              onClick={() => setIsOpen(false)}
            ></div>
          </dialog>,
          document.body
        )}
    </>
  );
}

export default AddAccountDialog;
