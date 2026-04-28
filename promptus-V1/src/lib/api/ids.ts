import { customAlphabet } from "nanoid";

const alphabet = "0123456789abcdefghijklmnopqrstuvwxyz";
const nano = customAlphabet(alphabet, 10);

export function generateId(prefix: string): string {
  return `${prefix}_${nano()}`;
}
