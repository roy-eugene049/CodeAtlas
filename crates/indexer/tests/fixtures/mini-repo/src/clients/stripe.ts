export class StripeClient {
  charge(amount: number): boolean {
    return amount > 0;
  }
}
