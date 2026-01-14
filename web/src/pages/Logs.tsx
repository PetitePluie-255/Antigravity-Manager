import { useEffect, useState, useMemo } from "react";
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
  Search,
  Activity,
  X,
  ArrowDownToLine,
  ArrowUpFromLine,
} from "lucide-react";
import { request } from "../api/client";
import { showToast } from "../components/common/ToastContainer";

// 日志条目类型
interface ProxyLogEntry {
  id: number;
  timestamp: number;
  method: string;
  url: string;
  account_email: string;
  model: string;
  tokens_in: number;
  tokens_out: number;
  latency_ms: number;
  status_code: number;
  error?: string;
  request_body?: string;
  response_body?: string;
}

// API 响应类型
interface LogQueryResponse {
  logs: ProxyLogEntry[];
  total: number;
}

// 快速过滤器定义
const quickFilters = [
  { label: "全部", value: "", icon: "📊" },
  { label: "仅错误", value: "error", icon: "❌" },
  { label: "聊天", value: "chat", icon: "💬" },
  { label: "Gemini", value: "gemini", icon: "✨" },
  { label: "Claude", value: "claude", icon: "🔮" },
  { label: "绘图", value: "image", icon: "🎨" },
];

function Logs() {
  const [logs, setLogs] = useState<ProxyLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [activeFilter, setActiveFilter] = useState("");
  const [selectedLog, setSelectedLog] = useState<ProxyLogEntry | null>(null);
  const limit = 20;

  const fetchLogs = async (offset = 0) => {
    setLoading(true);
    try {
      const result = await request<LogQueryResponse>(
        `/proxy/logs?limit=${limit}&offset=${offset}`
      );
      setLogs(result.logs || []);
      setTotal(result.total || 0);
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
      await request("/proxy/logs/clear", { method: "POST" });
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
    if (status >= 200 && status < 300) return "text-white bg-green-500";
    if (status >= 400 && status < 500) return "text-white bg-orange-500";
    if (status >= 500) return "text-white bg-red-500";
    return "text-gray-600 bg-gray-200";
  };

  const getMethodColor = (method: string) => {
    switch (method.toUpperCase()) {
      case "POST":
        return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400";
      case "GET":
        return "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400";
      case "PUT":
        return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400";
      case "DELETE":
        return "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400";
      default:
        return "bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300";
    }
  };

  // 格式化 JSON 显示
  const formatBody = (body?: string) => {
    if (!body) return <span className="text-gray-400 italic">Empty</span>;

    // 检查是否是 SSE 流数据
    if (body.startsWith("data:") || body.includes("\ndata:")) {
      return formatStreamData(body);
    }

    try {
      const obj = JSON.parse(body);
      return (
        <pre className="text-[11px] font-mono whitespace-pre-wrap text-gray-700 dark:text-gray-300 max-h-[300px] overflow-auto">
          {JSON.stringify(obj, null, 2)}
        </pre>
      );
    } catch {
      if (body === "[Stream Data]") {
        return (
          <span className="text-gray-400 italic">[流式响应 - 无详细数据]</span>
        );
      }
      return (
        <pre className="text-[11px] font-mono whitespace-pre-wrap text-gray-700 dark:text-gray-300 max-h-[300px] overflow-auto">
          {body}
        </pre>
      );
    }
  };

  // 格式化 SSE 流数据
  const formatStreamData = (body: string) => {
    const lines = body.split("\n");
    const chunks: any[] = [];
    let aggregatedContent = "";

    for (const line of lines) {
      if (line.startsWith("data:")) {
        const jsonStr = line.slice(5).trim();
        if (jsonStr === "[DONE]") continue;
        try {
          const obj = JSON.parse(jsonStr);
          chunks.push(obj);
          // 提取 delta content
          const delta =
            obj.choices?.[0]?.delta?.content ||
            obj.delta?.text ||
            obj.choices?.[0]?.text ||
            "";
          aggregatedContent += delta;
        } catch {
          // 忽略无法解析的行
        }
      }
    }

    if (chunks.length === 0) {
      return (
        <pre className="text-[11px] font-mono whitespace-pre-wrap text-gray-700 dark:text-gray-300 max-h-[300px] overflow-auto">
          {body}
        </pre>
      );
    }

    return (
      <div className="space-y-3">
        {/* 聚合内容预览 */}
        {aggregatedContent && (
          <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
            <div className="text-[10px] font-bold text-blue-600 dark:text-blue-400 uppercase mb-1 flex items-center gap-1">
              <span>📝</span> 聚合内容
            </div>
            <pre className="text-[11px] font-mono whitespace-pre-wrap text-gray-800 dark:text-gray-200 max-h-[150px] overflow-auto">
              {aggregatedContent}
            </pre>
          </div>
        )}

        {/* 流式 chunks 概要 */}
        <div className="bg-gray-50 dark:bg-base-200/50 border border-gray-200 dark:border-base-300 rounded-lg p-3">
          <div className="text-[10px] font-bold text-gray-500 dark:text-gray-400 uppercase mb-2 flex items-center gap-1">
            <span>📦</span> 流式数据块 ({chunks.length} 个)
          </div>
          <details className="text-[10px]">
            <summary className="cursor-pointer text-indigo-500 hover:text-indigo-600 dark:text-indigo-400 font-medium">
              点击展开原始数据块
            </summary>
            <pre className="mt-2 text-[10px] font-mono whitespace-pre-wrap text-gray-600 dark:text-gray-400 max-h-[200px] overflow-auto bg-white dark:bg-base-100 p-2 rounded border border-gray-100 dark:border-base-200">
              {chunks
                .map(
                  (chunk, i) =>
                    `--- Chunk ${i + 1} ---\n${JSON.stringify(
                      chunk,
                      null,
                      2
                    )}\n\n`
                )
                .join("")}
            </pre>
          </details>
        </div>
      </div>
    );
  };

  // 过滤日志
  const filteredLogs = useMemo(() => {
    return logs.filter((log) => {
      // 搜索过滤
      const matchesSearch =
        searchQuery === "" ||
        log.model.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.url.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.account_email.toLowerCase().includes(searchQuery.toLowerCase()) ||
        log.status_code.toString().includes(searchQuery);

      // 快速过滤器
      let matchesFilter = true;
      if (activeFilter === "error") {
        matchesFilter = log.status_code >= 400;
      } else if (activeFilter === "chat") {
        matchesFilter =
          log.url.toLowerCase().includes("chat") ||
          log.url.toLowerCase().includes("completion");
      } else if (activeFilter === "gemini") {
        matchesFilter = log.model.toLowerCase().includes("gemini");
      } else if (activeFilter === "claude") {
        matchesFilter = log.model.toLowerCase().includes("claude");
      } else if (activeFilter === "image") {
        matchesFilter =
          log.url.toLowerCase().includes("image") ||
          log.model.toLowerCase().includes("dall");
      }

      return matchesSearch && matchesFilter;
    });
  }, [logs, searchQuery, activeFilter]);

  // 统计数据
  const stats = useMemo(() => {
    const totalReqs = logs.length;
    const successCount = logs.filter(
      (l) => l.status_code >= 200 && l.status_code < 400
    ).length;
    const errorCount = logs.filter((l) => l.status_code >= 400).length;
    return { totalReqs, successCount, errorCount };
  }, [logs]);

  const totalPages = Math.ceil(total / limit);

  return (
    <div className="h-full w-full overflow-y-auto">
      <div className="p-5 space-y-4 max-w-7xl mx-auto">
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Activity className="w-6 h-6 text-indigo-500" />
          <h1 className="text-2xl font-bold text-gray-900 dark:text-base-content">
            API 监控看板
          </h1>
          <span className="text-sm text-gray-500 dark:text-gray-400">
            实时请求日志与分析
          </span>
        </div>

        {/* 工具栏 */}
        <div className="bg-white dark:bg-base-100 rounded-xl shadow-sm border border-gray-100 dark:border-base-200 p-4 space-y-3">
          {/* 第一行：搜索、统计、操作按钮 */}
          <div className="flex items-center gap-4 flex-wrap">
            {/* 搜索框 */}
            <div className="relative flex-1 min-w-[200px]">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input
                type="text"
                placeholder="搜索模型 (gemini, claude)、路径 (chat, images) 或状态码..."
                className="w-full pl-10 pr-4 py-2 border border-gray-200 dark:border-base-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 bg-gray-50 dark:bg-base-200 text-gray-900 dark:text-base-content"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>

            {/* 统计指标 */}
            <div className="flex gap-4 text-xs font-bold uppercase">
              <span className="text-blue-500">{stats.totalReqs} REQS</span>
              <span className="text-green-500">{stats.successCount} OK</span>
              <span className="text-red-500">{stats.errorCount} ERR</span>
            </div>

            {/* 操作按钮 */}
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
                className="px-3 py-1.5 bg-red-500 text-white text-xs font-medium rounded-lg hover:bg-red-600 transition-colors flex items-center gap-1.5 shadow-sm disabled:opacity-50"
                onClick={handleClear}
                disabled={logs.length === 0}
              >
                <Trash2 className="w-3.5 h-3.5" />
                清除
              </button>
            </div>
          </div>

          {/* 第二行：快速过滤器 */}
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-xs font-bold text-gray-400 uppercase">
              快速过滤:
            </span>
            {quickFilters.map((filter) => (
              <button
                key={filter.value}
                onClick={() => setActiveFilter(filter.value)}
                className={`px-3 py-1 text-xs rounded-full transition-all ${
                  activeFilter === filter.value
                    ? "bg-blue-500 text-white font-bold shadow-sm"
                    : "bg-gray-100 dark:bg-base-200 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-base-300"
                }`}
              >
                {filter.icon} {filter.label}
              </button>
            ))}
          </div>
        </div>

        {/* 日志列表 */}
        <div className="bg-white dark:bg-base-100 rounded-xl shadow-sm border border-gray-100 dark:border-base-200 overflow-hidden">
          {filteredLogs.length === 0 ? (
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
                    <th className="px-4 py-3 text-left font-medium">状态</th>
                    <th className="px-4 py-3 text-left font-medium">方法</th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Cpu className="w-3.5 h-3.5" />
                        模型
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">路径</th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <User className="w-3.5 h-3.5" />
                        账号
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Hash className="w-3.5 h-3.5" />
                        Token 消耗
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Timer className="w-3.5 h-3.5" />
                        耗时
                      </div>
                    </th>
                    <th className="px-4 py-3 text-left font-medium">
                      <div className="flex items-center gap-1">
                        <Clock className="w-3.5 h-3.5" />
                        时间
                      </div>
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-base-200">
                  {filteredLogs.map((log) => (
                    <tr
                      key={log.id}
                      className="hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors cursor-pointer"
                      onClick={() => setSelectedLog(log)}
                    >
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-bold ${getStatusColor(
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
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-bold ${getMethodColor(
                            log.method
                          )}`}
                        >
                          {log.method || "POST"}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="px-2 py-0.5 bg-indigo-50 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400 rounded text-xs font-medium truncate max-w-[150px] inline-block">
                          {log.model}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400 font-mono text-xs truncate max-w-[200px]">
                        {log.url || "/v1/chat/completions"}
                      </td>
                      <td className="px-4 py-3 text-gray-900 dark:text-base-content text-xs">
                        {log.account_email.split("@")[0]}
                      </td>
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400 font-mono text-xs">
                        <span className="text-blue-600 inline-flex items-center gap-0.5">
                          <ArrowDownToLine className="w-3 h-3" />
                          {log.tokens_in}
                        </span>
                        <span className="mx-1">·</span>
                        <span className="text-green-600 inline-flex items-center gap-0.5">
                          <ArrowUpFromLine className="w-3 h-3" />
                          {log.tokens_out}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-gray-600 dark:text-gray-400 font-mono">
                        {log.latency_ms}ms
                      </td>
                      <td className="px-4 py-3 text-gray-500 dark:text-gray-400 text-xs whitespace-nowrap">
                        {formatTime(log.timestamp)}
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
                第 {page + 1} / {totalPages} 页 · 共 {total} 条记录
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

      {/* 详情弹窗 */}
      {selectedLog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
          onClick={() => setSelectedLog(null)}
        >
          <div
            className="bg-white dark:bg-base-100 rounded-xl shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col overflow-hidden border border-gray-200 dark:border-base-300"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div className="px-4 py-3 border-b border-gray-100 dark:border-base-300 flex items-center justify-between bg-gray-50 dark:bg-base-200">
              <div className="flex items-center gap-3">
                <span
                  className={`px-2 py-1 rounded text-xs font-bold ${getStatusColor(
                    selectedLog.status_code
                  )}`}
                >
                  {selectedLog.status_code}
                </span>
                <span
                  className={`px-2 py-1 rounded text-xs font-bold ${getMethodColor(
                    selectedLog.method
                  )}`}
                >
                  {selectedLog.method || "POST"}
                </span>
                <span className="font-mono text-sm text-gray-700 dark:text-gray-300 truncate max-w-md">
                  {selectedLog.url || "/v1/chat/completions"}
                </span>
              </div>
              <button
                onClick={() => setSelectedLog(null)}
                className="btn btn-ghost btn-sm btn-circle text-gray-500 dark:text-gray-400"
              >
                <X size={18} />
              </button>
            </div>

            {/* Modal Content */}
            <div className="flex-1 overflow-y-auto p-4 space-y-6">
              {/* Metadata Section */}
              <div className="bg-gray-50 dark:bg-base-200 p-5 rounded-xl border border-gray-200 dark:border-base-300">
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-y-5 gap-x-10">
                  <div className="space-y-1.5">
                    <span className="block text-gray-500 dark:text-gray-400 uppercase font-black text-[10px] tracking-widest">
                      请求时间
                    </span>
                    <span className="font-mono font-semibold text-gray-900 dark:text-white text-sm">
                      {formatTime(selectedLog.timestamp)}
                    </span>
                  </div>
                  <div className="space-y-1.5">
                    <span className="block text-gray-500 dark:text-gray-400 uppercase font-black text-[10px] tracking-widest">
                      耗时
                    </span>
                    <span className="font-mono font-semibold text-gray-900 dark:text-white text-sm">
                      {selectedLog.latency_ms}ms
                    </span>
                  </div>
                  <div className="space-y-1.5">
                    <span className="block text-gray-500 dark:text-gray-400 uppercase font-black text-[10px] tracking-widest">
                      TOKEN 消耗 (输入/输出)
                    </span>
                    <div className="font-mono text-[11px] flex gap-2">
                      <span className="text-blue-700 dark:text-blue-300 bg-blue-100 dark:bg-blue-900/40 px-2.5 py-1 rounded-md border border-blue-200 dark:border-blue-800/50 font-bold inline-flex items-center gap-1">
                        <ArrowDownToLine className="w-3 h-3" />
                        {selectedLog.tokens_in}
                      </span>
                      <span className="text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900/40 px-2.5 py-1 rounded-md border border-green-200 dark:border-green-800/50 font-bold inline-flex items-center gap-1">
                        <ArrowUpFromLine className="w-3 h-3" />
                        {selectedLog.tokens_out}
                      </span>
                    </div>
                  </div>
                </div>
                <div className="mt-5 pt-5 border-t border-gray-200 dark:border-base-300">
                  <span className="block text-gray-500 dark:text-gray-400 uppercase font-black text-[10px] tracking-widest mb-2">
                    使用模型
                  </span>
                  <span className="font-mono font-black text-blue-600 dark:text-blue-400 break-all text-sm">
                    {selectedLog.model}
                  </span>
                </div>
              </div>

              {/* Payloads */}
              <div className="space-y-4">
                <div>
                  <h3 className="text-xs font-bold uppercase text-gray-400 mb-2 flex items-center gap-2">
                    请求报文 (REQUEST)
                  </h3>
                  <div className="bg-gray-50 dark:bg-base-200 rounded-lg p-3 border border-gray-100 dark:border-base-300 overflow-hidden">
                    {formatBody(selectedLog.request_body)}
                  </div>
                </div>
                <div>
                  <h3 className="text-xs font-bold uppercase text-gray-400 mb-2 flex items-center gap-2">
                    响应报文 (RESPONSE)
                  </h3>
                  <div className="bg-gray-50 dark:bg-base-200 rounded-lg p-3 border border-gray-100 dark:border-base-300 overflow-hidden">
                    {formatBody(selectedLog.response_body)}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default Logs;
