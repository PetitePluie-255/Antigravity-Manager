import { createPortal } from "react-dom";
import { useEffect, useState } from "react";
import { Wand2, RotateCcw, Trash2, X } from "lucide-react";
import {
  Account,
  DeviceProfile,
  DeviceProfileVersion,
} from "../../types/account";
import * as accountService from "../../services/accountService";
import { useTranslation } from "react-i18next";

interface DeviceFingerprintDialogProps {
  account: Account | null;
  onClose: () => void;
}

export default function DeviceFingerprintDialog({
  account,
  onClose,
}: DeviceFingerprintDialogProps) {
  const { t } = useTranslation();
  const [deviceProfiles, setDeviceProfiles] = useState<{
    current_storage?: DeviceProfile;
    history?: DeviceProfileVersion[];
    baseline?: DeviceProfile;
  } | null>(null);
  const [loadingDevice, setLoadingDevice] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [confirmProfile, setConfirmProfile] = useState<DeviceProfile | null>(
    null
  );
  const [confirmType, setConfirmType] = useState<
    "generate" | "restoreOriginal" | null
  >(null);

  const fetchDevice = async (target?: Account | null) => {
    if (!target) {
      setDeviceProfiles(null);
      return;
    }
    setLoadingDevice(true);
    try {
      const res = await accountService.getDeviceProfiles(target.id);
      setDeviceProfiles(res);
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.load_failed", "加载设备信息失败")
      );
    } finally {
      setLoadingDevice(false);
    }
  };

  useEffect(() => {
    fetchDevice(account);
  }, [account]);

  const handleGeneratePreview = async () => {
    setActionLoading("preview");
    try {
      const profile = await accountService.previewGenerateProfile();
      setConfirmProfile(profile);
      setConfirmType("generate");
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.generate_failed", "生成失败")
      );
    } finally {
      setActionLoading(null);
    }
  };

  const handleConfirmGenerate = async () => {
    if (!account || !confirmProfile) return;
    setActionLoading("generate");
    try {
      await accountService.bindDeviceProfileWithProfile(
        account.id,
        confirmProfile
      );
      setActionMessage(t("device.bound_success", "已生成并绑定"));
      setConfirmProfile(null);
      setConfirmType(null);
      await fetchDevice(account);
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.bind_failed", "绑定失败")
      );
    } finally {
      setActionLoading(null);
    }
  };

  const handleRestoreOriginalConfirm = () => {
    if (!deviceProfiles?.baseline) {
      setActionMessage(t("device.no_baseline", "未找到原始指纹"));
      return;
    }
    setConfirmProfile(deviceProfiles.baseline);
    setConfirmType("restoreOriginal");
  };

  const handleRestoreOriginal = async () => {
    if (!account) return;
    setActionLoading("restore");
    try {
      const msg = await accountService.restoreOriginalDevice();
      setActionMessage(msg);
      setConfirmProfile(null);
      setConfirmType(null);
      await fetchDevice(account);
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.restore_failed", "恢复失败")
      );
    } finally {
      setActionLoading(null);
    }
  };

  const handleRestoreVersion = async (versionId: string) => {
    if (!account) return;
    setActionLoading(`restore-${versionId}`);
    try {
      await accountService.restoreDeviceVersion(account.id, versionId);
      setActionMessage(t("device.version_restored", "已恢复该版本"));
      await fetchDevice(account);
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.restore_failed", "恢复失败")
      );
    } finally {
      setActionLoading(null);
    }
  };

  const handleDeleteVersion = async (
    versionId: string,
    isCurrent?: boolean
  ) => {
    if (!account) return;
    if (isCurrent) {
      setActionMessage(
        t("device.cannot_delete_current", "无法删除当前使用的版本")
      );
      return;
    }
    setActionLoading(`delete-${versionId}`);
    try {
      await accountService.deleteDeviceVersion(account.id, versionId);
      setActionMessage(t("device.version_deleted", "已删除"));
      await fetchDevice(account);
    } catch (e: any) {
      setActionMessage(
        typeof e === "string" ? e : t("device.delete_failed", "删除失败")
      );
    } finally {
      setActionLoading(null);
    }
  };

  const renderProfile = (profile?: DeviceProfile) => {
    if (!profile)
      return (
        <div className="text-xs text-gray-400">
          {t("device.not_bound", "未绑定")}
        </div>
      );
    return (
      <div className="text-[10px] font-mono text-gray-600 dark:text-gray-400 space-y-0.5">
        <div>
          <span className="text-gray-400">machineId:</span> {profile.machine_id}
        </div>
        <div>
          <span className="text-gray-400">macMachineId:</span>{" "}
          {profile.mac_machine_id}
        </div>
        <div>
          <span className="text-gray-400">devDeviceId:</span>{" "}
          {profile.dev_device_id}
        </div>
        <div>
          <span className="text-gray-400">sqmId:</span> {profile.sqm_id}
        </div>
      </div>
    );
  };

  if (!account) return null;

  return createPortal(
    <div className="modal modal-open z-[100]">
      <div className="modal-box max-w-2xl bg-white dark:bg-base-100 rounded-2xl shadow-2xl p-6 relative">
        <button
          className="absolute right-4 top-4 btn btn-sm btn-circle btn-ghost"
          onClick={onClose}
        >
          <X size={18} />
        </button>
        <h3 className="font-bold text-lg text-gray-900 dark:text-base-content mb-4">
          {t("device.dialog_title", "设备指纹管理")} - {account.email}
        </h3>

        {actionMessage && (
          <div className="alert alert-info mb-4 py-2 text-sm">
            {actionMessage}
            <button
              className="btn btn-xs btn-ghost ml-auto"
              onClick={() => setActionMessage(null)}
            >
              ✕
            </button>
          </div>
        )}

        <div className="space-y-4">
          {/* 操作按钮 */}
          <div className="flex gap-2 flex-wrap">
            <button
              className="btn btn-sm btn-outline gap-2"
              onClick={handleGeneratePreview}
              disabled={!!actionLoading}
            >
              <Wand2 size={14} />
              {t("device.generate_new", "生成新指纹")}
            </button>
            <button
              className="btn btn-sm btn-outline gap-2"
              onClick={handleRestoreOriginalConfirm}
              disabled={!!actionLoading || !deviceProfiles?.baseline}
            >
              <RotateCcw size={14} />
              {t("device.restore_original", "恢复原始指纹")}
            </button>
          </div>

          {/* 当前绑定 */}
          <div className="p-3 rounded-xl border border-gray-100 dark:border-base-200 bg-gray-50 dark:bg-base-200/50">
            <div className="text-xs font-semibold text-gray-700 dark:text-gray-200 mb-2">
              {t("device.current_binding", "当前绑定")}
            </div>
            {loadingDevice ? (
              <div className="text-xs text-gray-400">
                {t("common.loading", "加载中...")}
              </div>
            ) : (
              renderProfile(
                deviceProfiles?.history?.find((h) => h.is_current)?.profile
              )
            )}
          </div>

          {/* 历史版本 */}
          <div className="p-3 rounded-xl border border-gray-100 dark:border-base-200 bg-white dark:bg-base-100">
            <div className="text-xs font-semibold text-gray-700 dark:text-gray-200 mb-2">
              {t("device.history", "历史指纹")}
            </div>
            {loadingDevice ? (
              <div className="text-xs text-gray-400">
                {t("common.loading", "加载中...")}
              </div>
            ) : (
              <div className="space-y-2 max-h-60 overflow-y-auto">
                {deviceProfiles?.history?.map((v) => (
                  <div
                    key={v.id}
                    className="flex items-start justify-between p-2 rounded-lg border border-gray-100 dark:border-base-200 hover:border-indigo-200 dark:hover:border-indigo-500/40 transition-colors"
                  >
                    <div className="text-[11px] text-gray-600 dark:text-gray-300 flex-1">
                      <div className="font-semibold">
                        {v.label || v.id}
                        {v.is_current && (
                          <span className="ml-2 text-[10px] text-blue-500">
                            {t("device.current", "当前")}
                          </span>
                        )}
                      </div>
                      {v.created_at > 0 && (
                        <div className="text-[10px] text-gray-400">
                          {new Date(v.created_at * 1000).toLocaleString()}
                        </div>
                      )}
                      <div className="mt-1 text-[10px] font-mono text-gray-500">
                        <div>machineId: {v.profile.machine_id}</div>
                      </div>
                    </div>
                    <div className="flex gap-2 ml-2">
                      <button
                        className="btn btn-xs btn-outline"
                        disabled={
                          actionLoading === `restore-${v.id}` || v.is_current
                        }
                        onClick={() => handleRestoreVersion(v.id)}
                        title={t("device.restore_version", "恢复此版本")}
                      >
                        {t("common.restore", "恢复")}
                      </button>
                      {!v.is_current && (
                        <button
                          className="btn btn-xs btn-outline btn-error"
                          disabled={actionLoading === `delete-${v.id}`}
                          onClick={() =>
                            handleDeleteVersion(v.id, v.is_current)
                          }
                          title={t("device.delete_version", "删除此版本")}
                        >
                          <Trash2 size={14} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
                {(!deviceProfiles?.history ||
                  deviceProfiles.history.length === 0) && (
                  <div className="text-xs text-gray-400">
                    {t("device.no_history", "暂无历史")}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
      <div
        className="modal-backdrop bg-black/40 backdrop-blur-sm"
        onClick={onClose}
      ></div>

      {/* Confirm Dialog */}
      {confirmProfile && confirmType && (
        <ConfirmDialog
          profile={confirmProfile}
          type={confirmType}
          onCancel={() => {
            if (actionLoading) return;
            setConfirmProfile(null);
            setConfirmType(null);
          }}
          onConfirm={
            confirmType === "generate"
              ? handleConfirmGenerate
              : handleRestoreOriginal
          }
          loading={!!actionLoading}
        />
      )}
    </div>,
    document.body
  );
}

// Confirm Dialog Component
function ConfirmDialog({
  profile,
  type,
  onConfirm,
  onCancel,
  loading,
}: {
  profile: DeviceProfile;
  type: "generate" | "restoreOriginal";
  onConfirm: () => void;
  onCancel: () => void;
  loading?: boolean;
}) {
  const { t } = useTranslation();
  const title =
    type === "generate"
      ? t("device.confirm_generate_title", "确认生成并绑定？")
      : t("device.confirm_restore_title", "确认恢复原始指纹？");
  const desc =
    type === "generate"
      ? t(
          "device.confirm_generate_desc",
          "将生成一套新的设备指纹并设置为当前指纹。确认继续？"
        )
      : t(
          "device.confirm_restore_desc",
          "将恢复为原始指纹并覆盖当前指纹。确认继续？"
        );

  return createPortal(
    <div className="modal modal-open z-[140]">
      <div className="modal-box max-w-sm bg-white dark:bg-base-100 rounded-2xl shadow-2xl p-6 text-center">
        <h3 className="font-bold text-lg text-gray-900 dark:text-base-content mb-1">
          {title}
        </h3>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">{desc}</p>
        <div className="text-xs font-mono text-gray-600 dark:text-gray-300 bg-gray-50 dark:bg-base-200/60 border border-gray-100 dark:border-base-200 rounded-lg p-3 text-left space-y-1">
          <div>
            <span className="font-semibold">machineId:</span>{" "}
            {profile.machine_id}
          </div>
          <div>
            <span className="font-semibold">macMachineId:</span>{" "}
            {profile.mac_machine_id}
          </div>
          <div>
            <span className="font-semibold">devDeviceId:</span>{" "}
            {profile.dev_device_id}
          </div>
          <div>
            <span className="font-semibold">sqmId:</span> {profile.sqm_id}
          </div>
        </div>
        <div className="mt-5 flex gap-3 justify-center">
          <button
            className="btn btn-sm min-w-[100px]"
            onClick={onCancel}
            disabled={!!loading}
          >
            {t("common.cancel", "取消")}
          </button>
          <button
            className="btn btn-sm btn-primary min-w-[100px]"
            onClick={onConfirm}
            disabled={!!loading}
          >
            {loading
              ? t("common.processing", "处理中...")
              : t("common.confirm", "确认")}
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-black/30" onClick={onCancel}></div>
    </div>,
    document.body
  );
}
