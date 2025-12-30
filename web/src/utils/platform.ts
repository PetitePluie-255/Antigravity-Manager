/**
 * Platform utilities
 * Web-only version
 */

export const isTauri = (): boolean => false;

export const getPlatform = async (): Promise<string> => {
  return "web";
};
