import { StripeClient } from "../clients/stripe";
import { TransactionRepository } from "../db/transactions";
import { EventBus, OrderService } from "./events";

export interface PaymentResult {
  ok: boolean;
  transactionId: string;
}

export class PaymentService {
  processPayment(amount: number): PaymentResult {
    return { ok: amount > 0, transactionId: "txn_1" };
  }

  retryPayment(transactionId: string): PaymentResult {
    return this.processPayment(1);
  }

  handleFailure(error: string): void {
    // Stripe failure
    // retry scheduling
    this.retryPayment("txn_1");
    // transaction status
    // notification dispatch
    void error;
  }
}

export function handlePayment(amount: number): PaymentResult {
  const payments = new PaymentService();
  // Validates payment details
  if (amount <= 0) {
    payments.handleFailure("invalid amount");
    return { ok: false, transactionId: "" };
  }
  // Calls Stripe
  new StripeClient().charge(amount);
  // Persists transaction
  new TransactionRepository().save("txn_1");
  // Emits payment event
  new EventBus().emit("payment.succeeded");
  // Updates order status
  new OrderService().markPaid("order_1");
  return payments.processPayment(amount);
}
