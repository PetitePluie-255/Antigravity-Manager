import { useEffect, useState } from "react";
import {
  FileText,
  RefreshCw,
  Trash2,
  ChevronLeft,
  ChevronRight,
  Clock,
  User,
  Cpu,
  Hash,
  Timer,
  AlertCircle,
} from "lucide-react";
import { apiCall } from "../utils/platform";
import { showToast } from "../components/common/ToastContainer";

// 日志条目类型
interface ProxyLogEntry {
  id: number;
  timestamp: number;
  account_email: string;
  model: string;
  tokens_in: number;
  tokens_out: number;
  latency_ms: number;
  status_code: number;
  error?: string;
}

// API 响应类型
interface LogQueryResponse {
  logs: ProxyLogEntry[];
  total: number;
}

function Logs() {
  const [logs, setLogs] = useState<ProxyLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const limit = 20;

  const fetchLogs = async (offset = 0) => {
    setLoading(true);
    try {
      const result = await apiCall<LogQueryResponse>("get_proxy_logs", {
        limit,
        offset,
      });
      setLogs(result.logs);
      setTotal(result.total);
    } catch (error) {
      console.error("Failed to fetch logs:", error);
      showToast(`加载日志失败: ${error}`, "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs(page * limit);
  }, [page]);

  const handleRefresh = () => {
    fetchLogs(page * limit);
    showToast("日志已刷新", "success");
  };

  const handleClear = async () => {
    if (!confirm("确定要清除所有日志吗？此操作不可恢复。")) {
      return;
    }
    try {
      await apiCall("clear_proxy_logs", {});
      setLogs([]);
      setTotal(0);
      setPage(0);
      showToast("日志已清除", "success");
    } catch (error) {
      console.error("Failed to clear logs:", error);
      showToast(`清除日志失败: ${error}`, "error");
    }
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return "text-green-600 bg-green-50";
    if (status >= 400 && status < 500) return "text-orange-600 bg-orange-50";
    if (status >= 500) return "text-red-600 bg-red-50";
    return "text-gray-600 bg-gray-50";
  };

  const totalPages = Math.ceil(total / limit);

  return (
    <div className="h-full w-full overflow-y-auto">
      <div className="p-5 space-y-4 max-w-7xl mx-auto">
        {/* 标题和操作按钮 */}
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-2">
            <FileText className="w-6 h-6 text-indigo-500" />
            <h1 className="text-2xl font-bold text-gray-900 dark:text-base-content">
              代理日志
            </h1>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              ({total} 条记录)
            </span>
          </div>
          <div className="flex gap-2">
            <button
              className={`px-3 py-1.5 bg-blue-500 text-white text-xs font-medium rounded-lg hover:bg-blue-600 transition-colors flex items-center gap-1.5 shadow-sm ${
                loading ? "opacity-70 cursor-not-allowed" : ""
              }`}
              onClick={handleRefresh}
              disabled={loading}
            >
              <RefreshCw
                className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`}
              />
              刷新
            </button>
            <button
              className="px-3 py-1.5 bg-red-500 text-white text-xs font-medium rounded-lg hover:bg-red-600 transition-colors flex items-center gap-1.5 shadow-sm"
              onClick={handleClear}
              disabled={logs.length === 0}
            >
              <Trash2 className="w-3.5 h-3.5" />
              清除
            </button>
          </div>
        </div>

        {/* 日志列表 */}
        <div className="bg-white dark:bg-base-100 rounded-xl shadow-sm border border-gray-100 dark:border-base-200 overflow-hidden">
          {logs.length === 0 ? (
            <div className="p-12 text-center text-gray-500 dark:text-gray-400">
              <FileText className="w-12 h-12 mx-auto mb-3 opacity-30" />
              <p>暂无日志记录</p>
              <p className="text-sm mt-1">代理请求日志将显示在这里</p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead className="bg-gray-50 dark:bg-base-200 text-gray-600 dark:text-gray-400">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Clock className="w-3.5 h-3.5" />
                        时间
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <User className="w-3.5 h-3.5" />
                        账号
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Cpu className="w-3.5 h-3.5" />
                        模型
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Hash className="w-3.5 h-3.5" />
                        Tokens
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Timer className="w-3.5 h-3.5" />
                        延迟
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">状态</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-base-200">
                  {logs.map((log) => (
                    <tr
                      key={log.id}
                      className="hover:bg-gray-50 dark:hover:bg-base-200 transition-colors"
                    >
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400 whitespace-nowrap">
                        {formatTime(log.timestamp)}
                      </td>
                      <td className="px-4 py-3 text-gray-900 dark:text-base-content">
                        {log.account_email.split("@")[0]}
                      </td>
                      <td className="px-4 py-3">
                        <span className="px-2 py-0.5 bg-indigo-50 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400 rounded text-xs font-medium">
                          {log.model}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400">
                        <span className="text-green-600">{log.tokens_in}</span>
                        <span className="mx-1">/</span>
                        <span className="text-blue-600">{log.tokens_out}</span>
                      </td>
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400">
                        {log.latency_ms}ms
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-medium ${getStatusColor(
                            log.status_code
                          )}`}
                        >
                          {log.status_code}
                        </span>
                        {log.error && (
                          <span className="ml-2 text-red-500" title={log.error}>
                            <AlertCircle className="w-3.5 h-3.5 inline" />
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {/* 分页 */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between px-4 py-3 border-t border-gray-100 dark:border-base-200">
              <div className="text-sm text-gray-500 dark:text-gray-400">
                第 {page + 1} / {totalPages} 页
              </div>
              <div className="flex gap-2">
                <button
                  className="px-3 py-1 text-sm border border-gray-200 dark:border-base-300 rounded hover:bg-gray-50 dark:hover:bg-base-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                  onClick={() => setPage((p) => p - 1)}
                  disabled={page === 0}
                >
                  <ChevronLeft className="w-4 h-4" />
                  上一页
                </button>
                <button
                  className="px-3 py-1 text-sm border border-gray-200 dark:border-base-300 rounded hover:bg-gray-50 dark:hover:bg-base-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                  onClick={() => setPage((p) => p + 1)}
                  disabled={page >= totalPages - 1}
                >
                  下一页
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default Logs;
