export interface Account {
  id: string;
  email: string;
  name?: string;
  token: TokenData;
  quota?: QuotaData;
  created_at: number;
  last_used: number;
  device_profile?: DeviceProfile;
  device_history?: DeviceProfileVersion[];
}

export interface TokenData {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  expiry_timestamp: number;
  token_type: string;
  email?: string;
}

export interface QuotaData {
  models: ModelQuota[];
  last_updated: number;
  is_forbidden?: boolean;
  subscription_tier?: string; // 订阅类型: FREE/PRO/ULTRA
}

export interface ModelQuota {
  name: string;
  percentage: number;
  reset_time: string;
}

export interface DeviceProfile {
  machine_id: string;
  mac_machine_id: string;
  dev_device_id: string;
  sqm_id: string;
}

export interface DeviceProfileVersion {
  id: string;
  label?: string;
  profile: DeviceProfile;
  created_at: number;
  is_current?: boolean;
}

export interface DeviceProfilesResponse {
  current_storage?: DeviceProfile;
  history?: DeviceProfileVersion[];
  baseline?: DeviceProfile;
}
