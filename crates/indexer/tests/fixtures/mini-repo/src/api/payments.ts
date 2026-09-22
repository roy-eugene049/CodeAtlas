import { handlePayment } from "../services/payment";

export function paymentHandler(amount: number) {
  return handlePayment(amount);
}
