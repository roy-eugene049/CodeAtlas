import { handlePayment } from "../services/payment";

export function PayButton() {
  return handlePayment(20);
}
