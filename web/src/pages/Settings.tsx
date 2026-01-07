import { useState, useEffect } from "react";
import { Save, Github, User, ExternalLink, Sparkles } from "lucide-react";
import { useConfigStore } from "../stores/useConfigStore";
import { AppConfig } from "../types/config";
import { showToast } from "../components/common/ToastContainer";
import { useTranslation } from "react-i18next";
import { request } from "../api/client";

function Settings() {
  const { t } = useTranslation();
  const { config, loadConfig, saveConfig } = useConfigStore();
  const [activeTab, setActiveTab] = useState<
    "general" | "account" | "proxy" | "advanced" | "about"
  >("general");
  const [formData, setFormData] = useState<AppConfig>({
    language: "zh",
    theme: "system",
    auto_refresh: false,
    refresh_interval: 15,
    auto_sync: false,
    sync_interval: 5,
    proxy: {
      enabled: false,
      port: 8080,
      api_key: "",
      auto_start: false,
      request_timeout: 120,
      upstream_proxy: {
        enabled: false,
        url: "",
      },
    },
  });

  const [isClearLogsOpen, setIsClearLogsOpen] = useState(false);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (config) {
      setFormData(config);
    }
  }, [config]);

  const handleSave = async () => {
    try {
      await saveConfig(formData);
      showToast(t("common.saved"), "success");
    } catch (error) {
      showToast(`${t("common.error")}: ${error}`, "error");
    }
  };

  const confirmClearLogs = async () => {
    try {
      await request("/logs", { method: "DELETE" });
      showToast(t("settings.advanced.logs_cleared"), "success");
    } catch (error) {
      showToast(`${t("common.error")}: ${error}`, "error");
    }
    setIsClearLogsOpen(false);
  };

  return (
    <div className="h-full w-full overflow-y-auto">
      <div className="p-5 space-y-4 max-w-7xl mx-auto">
        {/* 顶部工具栏：Tab 导航和保存按钮 */}
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-1 bg-gray-100 dark:bg-base-200 rounded-full p-1 w-fit">
            {[
              { id: "general", label: t("settings.tabs.general") },
              { id: "account", label: t("settings.tabs.account") },
              { id: "proxy", label: t("settings.tabs.proxy") },
              { id: "advanced", label: t("settings.tabs.advanced") },
              { id: "about", label: t("settings.tabs.about") },
            ].map((tab) => (
              <button
                key={tab.id}
                className={`px-6 py-2 rounded-full text-sm font-medium transition-all ${
                  activeTab === tab.id
                    ? "bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                    : "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
                }`}
                onClick={() => setActiveTab(tab.id as any)}
              >
                {tab.label}
              </button>
            ))}
          </div>

          <button
            className="px-4 py-2 bg-blue-500 text-white text-sm rounded-lg hover:bg-blue-600 transition-colors flex items-center gap-2 shadow-sm"
            onClick={handleSave}
          >
            <Save className="w-4 h-4" />
            {t("settings.save")}
          </button>
        </div>

        {/* 设置表单 */}
        <div className="bg-white dark:bg-base-100 rounded-2xl p-6 shadow-sm border border-gray-100 dark:border-base-200">
          {/* 通用设置 */}
          {activeTab === "general" && (
            <div className="space-y-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-base-content">
                {t("settings.general.title")}
              </h2>

              {/* 语言选择 */}
              <div>
                <label className="block text-sm font-medium text-gray-900 dark:text-base-content mb-2">
                  {t("settings.general.language")}
                </label>
                <select
                  className="w-full px-4 py-4 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                  value={formData.language}
                  onChange={(e) =>
                    setFormData({ ...formData, language: e.target.value })
                  }
                >
                  <option value="zh">简体中文</option>
                  <option value="en">English</option>
                </select>
              </div>

              {/* 主题选择 */}
              <div>
                <label className="block text-sm font-medium text-gray-900 dark:text-base-content mb-2">
                  {t("settings.general.theme")}
                </label>
                <select
                  className="w-full px-4 py-4 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                  value={formData.theme}
                  onChange={(e) =>
                    setFormData({ ...formData, theme: e.target.value })
                  }
                >
                  <option value="light">
                    {t("settings.general.theme_light")}
                  </option>
                  <option value="dark">
                    {t("settings.general.theme_dark")}
                  </option>
                  <option value="system">
                    {t("settings.general.theme_system")}
                  </option>
                </select>
              </div>
            </div>
          )}

          {/* 账号设置 - 保留配置字段，即使前端逻辑尚未完全复刻 */}
          {activeTab === "account" && (
            <div className="space-y-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-base-content">
                {t("settings.account.title")}
              </h2>
              {/* 自动刷新配额 */}
              <div className="flex items-center justify-between p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <div>
                  <div className="font-medium text-gray-900 dark:text-base-content">
                    {t("settings.account.auto_refresh")}
                  </div>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    className="sr-only peer"
                    checked={formData.auto_refresh}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        auto_refresh: e.target.checked,
                      })
                    }
                  />
                  <div className="w-11 h-6 bg-gray-200 dark:bg-base-300 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>

              {/* 刷新间隔 */}
              {formData.auto_refresh && (
                <div>
                  <label className="block text-sm font-medium text-gray-900 dark:text-base-content mb-2">
                    {t("settings.account.refresh_interval")}
                  </label>
                  <input
                    type="number"
                    className="w-32 px-4 py-4 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                    min="1"
                    max="60"
                    value={formData.refresh_interval}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        refresh_interval: parseInt(e.target.value),
                      })
                    }
                  />
                </div>
              )}
            </div>
          )}

          {/* 代理设置 */}
          {activeTab === "proxy" && (
            <div className="space-y-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-base-content">
                {t("settings.tabs.proxy")}
              </h2>

              {/* 上游代理配置 */}
              <div className="p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <h3 className="text-md font-semibold text-gray-900 dark:text-base-content mb-3 flex items-center gap-2">
                  <Sparkles size={18} className="text-blue-500" />
                  {t("proxy.config.upstream_proxy.title")}
                </h3>
                <div className="space-y-4">
                  <div className="flex items-center">
                    <label className="flex items-center cursor-pointer gap-3">
                      <input
                        type="checkbox"
                        className="checkbox checkbox-sm checkbox-primary"
                        checked={
                          formData.proxy?.upstream_proxy?.enabled || false
                        }
                        onChange={(e) =>
                          setFormData({
                            ...formData,
                            proxy: {
                              ...formData.proxy,
                              upstream_proxy: {
                                ...formData.proxy.upstream_proxy,
                                enabled: e.target.checked,
                              },
                            },
                          })
                        }
                      />
                      <span className="text-sm font-medium text-gray-900 dark:text-base-content">
                        {t("proxy.config.upstream_proxy.enable")}
                      </span>
                    </label>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      {t("proxy.config.upstream_proxy.url")}
                    </label>
                    <input
                      type="text"
                      value={formData.proxy?.upstream_proxy?.url || ""}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          proxy: {
                            ...formData.proxy,
                            upstream_proxy: {
                              ...formData.proxy.upstream_proxy,
                              url: e.target.value,
                            },
                          },
                        })
                      }
                      placeholder={t(
                        "proxy.config.upstream_proxy.url_placeholder"
                      )}
                      className="w-full px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                    />
                  </div>
                </div>
              </div>

              {/* z.ai 配置 */}
              <div className="p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <h3 className="text-md font-semibold text-gray-900 dark:text-base-content mb-3 flex items-center gap-2">
                  <Sparkles size={18} className="text-purple-500" />
                  z.ai 配置
                </h3>
                <div className="space-y-4">
                  <div className="flex items-center">
                    <label className="flex items-center cursor-pointer gap-3">
                      <input
                        type="checkbox"
                        className="checkbox checkbox-sm checkbox-primary"
                        checked={formData.proxy?.zai?.enabled || false}
                        onChange={(e) =>
                          setFormData({
                            ...formData,
                            proxy: {
                              ...formData.proxy,
                              zai: {
                                ...formData.proxy.zai,
                                enabled: e.target.checked,
                              },
                            },
                          })
                        }
                      />
                      <span className="text-sm font-medium text-gray-900 dark:text-base-content">
                        启用 z.ai
                      </span>
                    </label>
                  </div>

                  {formData.proxy?.zai?.enabled && (
                    <>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                          API Key
                        </label>
                        <input
                          type="password"
                          value={formData.proxy?.zai?.api_key || ""}
                          onChange={(e) =>
                            setFormData({
                              ...formData,
                              proxy: {
                                ...formData.proxy,
                                zai: {
                                  ...formData.proxy.zai,
                                  api_key: e.target.value,
                                },
                              },
                            })
                          }
                          placeholder="输入 z.ai API Key"
                          className="w-full px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                        />
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                          调度模式
                        </label>
                        <select
                          value={formData.proxy?.zai?.dispatch_mode || "off"}
                          onChange={(e) =>
                            setFormData({
                              ...formData,
                              proxy: {
                                ...formData.proxy,
                                zai: {
                                  ...formData.proxy.zai,
                                  dispatch_mode: e.target.value,
                                },
                              },
                            })
                          }
                          className="w-full px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                        >
                          <option value="off">关闭</option>
                          <option value="exclusive">
                            独占模式 (仅使用 z.ai)
                          </option>
                          <option value="pooled">
                            池化模式 (与 Google 轮询)
                          </option>
                          <option value="fallback">
                            回退模式 (Google 不可用时使用)
                          </option>
                        </select>
                      </div>
                    </>
                  )}
                </div>
              </div>

              {/* 调度模式配置 */}
              <div className="p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <h3 className="text-md font-semibold text-gray-900 dark:text-base-content mb-3 flex items-center gap-2">
                  <Sparkles size={18} className="text-green-500" />
                  账号调度模式
                </h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      调度策略
                    </label>
                    <select
                      value={formData.proxy?.scheduling?.mode || "balance"}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          proxy: {
                            ...formData.proxy,
                            scheduling: {
                              ...formData.proxy.scheduling,
                              mode: e.target.value,
                            },
                          },
                        })
                      }
                      className="w-full px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                    >
                      <option value="cache_first">
                        缓存优先 (最大化 Prompt Cache)
                      </option>
                      <option value="balance">平衡模式 (推荐)</option>
                      <option value="performance_first">
                        性能优先 (纯轮询)
                      </option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      最大等待时间 (秒)
                    </label>
                    <input
                      type="number"
                      min="1"
                      max="300"
                      value={formData.proxy?.scheduling?.max_wait_seconds || 60}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          proxy: {
                            ...formData.proxy,
                            scheduling: {
                              ...formData.proxy.scheduling,
                              max_wait_seconds: parseInt(e.target.value) || 60,
                            },
                          },
                        })
                      }
                      className="w-32 px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                    />
                  </div>
                </div>
              </div>

              {/* 安全模式 */}
              <div className="p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <h3 className="text-md font-semibold text-gray-900 dark:text-base-content mb-3 flex items-center gap-2">
                  <Sparkles size={18} className="text-orange-500" />
                  安全模式
                </h3>
                <div>
                  <select
                    value={formData.proxy?.auth_mode || "auto"}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        proxy: {
                          ...formData.proxy,
                          auth_mode: e.target.value,
                        },
                      })
                    }
                    className="w-full px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-base-content bg-gray-50 dark:bg-base-200"
                  >
                    <option value="auto">自动 (根据访问来源智能判断)</option>
                    <option value="off">关闭 (不验证 API Key)</option>
                    <option value="strict">
                      严格模式 (所有请求都需要验证)
                    </option>
                    <option value="all_except_health">除健康检查外验证</option>
                  </select>
                </div>
              </div>
            </div>
          )}

          {/* 高级设置 */}
          {activeTab === "advanced" && (
            <div className="space-y-4">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-base-content">
                {t("settings.advanced.title")}
              </h2>
              {/* 默认导出路径 - Web 端通常无法设置默认下载路径，这里显示提示或禁用 */}
              <div className="alert alert-info py-2 text-sm">
                Web Note: Files are downloaded to your browser's default
                download location.
              </div>

              <div className="border-t border-gray-200 dark:border-base-200 pt-4">
                <h3 className="font-medium text-gray-900 dark:text-base-content mb-3">
                  {t("settings.advanced.logs_title")}
                </h3>
                <div className="flex items-center gap-4">
                  <button
                    className="px-4 py-2 border border-gray-300 dark:border-base-300 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-base-200 transition-colors"
                    onClick={() => setIsClearLogsOpen(true)}
                  >
                    {t("settings.advanced.clear_logs")}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* About */}
          {activeTab === "about" && (
            <div className="flex flex-col h-full animate-in fade-in duration-500 items-center justify-center space-y-8">
              <div className="text-center space-y-4">
                <h3 className="text-3xl font-black text-gray-900 dark:text-base-content tracking-tight mb-2">
                  Antigravity Tools
                </h3>
                <div className="badge badge-primary badge-outline gap-2 font-mono">
                  v3.3.5 (Web)
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full max-w-lg px-4">
                {/* Author Card */}
                <div className="bg-white dark:bg-base-100 p-4 rounded-2xl border border-gray-100 dark:border-base-300 shadow-sm flex flex-col items-center text-center gap-3">
                  <User className="w-6 h-6 text-blue-500" />
                  <div>
                    <div className="font-bold text-gray-900 dark:text-base-content">
                      Ctrler
                    </div>
                  </div>
                </div>
                {/* GitHub Card */}
                <a
                  href="https://github.com/lbjlaq/Antigravity-Manager"
                  target="_blank"
                  rel="noreferrer"
                  className="bg-white dark:bg-base-100 p-4 rounded-2xl border border-gray-100 dark:border-base-300 shadow-sm hover:shadow-md transition-all flex flex-col items-center text-center gap-3 cursor-pointer"
                >
                  <Github className="w-6 h-6 text-gray-900 dark:text-white" />
                  <div className="flex items-center gap-1 font-bold text-gray-900 dark:text-base-content">
                    <span>GitHub</span>
                    <ExternalLink className="w-3 h-3 text-gray-400" />
                  </div>
                </a>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Clear Logs Dialog */}
      {isClearLogsOpen && (
        <div className="modal modal-open">
          <div className="modal-box">
            <h3 className="font-bold text-lg">
              {t("settings.advanced.logs_title")}
            </h3>
            <p className="py-4">{t("settings.advanced.clear_logs_confirm")}</p>
            <div className="modal-action">
              <button className="btn" onClick={() => setIsClearLogsOpen(false)}>
                {t("common.cancel")}
              </button>
              <button className="btn btn-error" onClick={confirmClearLogs}>
                {t("common.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default Settings;
