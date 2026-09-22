import { request } from "./http";

export const MAX_RETRIES = 3;

export function createClient(baseUrl) {
  return {
    get(path) {
      return request(baseUrl + path);
    },
  };
}
