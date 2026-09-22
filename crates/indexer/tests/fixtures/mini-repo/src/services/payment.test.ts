import { handlePayment } from "./payment";

export function charges_a_valid_amount() {
  return handlePayment(10);
}
