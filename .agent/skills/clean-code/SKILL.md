---
name: clean-code
description: Clean code principles and refactoring patterns. Use when writing new code, reviewing code quality, or refactoring existing code.
---

# Clean Code Skill

> Code that reads like well-written prose.

## When to Activate

- Writing new functions/classes
- Code review sessions
- Refactoring legacy code
- Establishing team standards
- Reducing technical debt

## Core Principles

### 1. Meaningful Names

```typescript
// ❌ BAD: Unclear abbreviations
const d = new Date();
const u = getU(id);
const list = getData();

// ✅ GOOD: Descriptive names
const currentDate = new Date();
const user = getUserById(id);
const activeMembers = getActiveMembers();

// ❌ BAD: Generic names
function process(data) {}
function handle(items) {}
function doIt() {}

// ✅ GOOD: Intent-revealing names
function validateUserEmail(email: string) {}
function calculateOrderTotal(items: OrderItem[]) {}
function sendWelcomeNotification(user: User) {}
```

### 2. Small Functions (SRP)

```typescript
// ❌ BAD: Function does too much
function processOrder(order: Order) {
  // Validate
  if (!order.items.length) throw new Error('Empty');
  if (!order.customer) throw new Error('No customer');
  
  // Calculate totals
  let subtotal = 0;
  for (const item of order.items) {
    subtotal += item.price * item.quantity;
  }
  const tax = subtotal * 0.1;
  const total = subtotal + tax;
  
  // Save to database
  db.orders.create({ ...order, subtotal, tax, total });
  
  // Send notification
  email.send(order.customer.email, 'Order confirmed');
  
  // Update inventory
  for (const item of order.items) {
    db.inventory.decrement(item.productId, item.quantity);
  }
  
  return { subtotal, tax, total };
}

// ✅ GOOD: Single responsibility
function processOrder(order: Order): OrderResult {
  validateOrder(order);
  const totals = calculateTotals(order);
  const savedOrder = saveOrder(order, totals);
  notifyCustomer(order.customer, savedOrder);
  updateInventory(order.items);
  return { orderId: savedOrder.id, ...totals };
}

function validateOrder(order: Order): void {
  if (!order.items.length) throw new Error('Order has no items');
  if (!order.customer) throw new Error('Order has no customer');
}

function calculateTotals(order: Order): OrderTotals {
  const subtotal = order.items.reduce(
    (sum, item) => sum + item.price * item.quantity, 0
  );
  const tax = subtotal * TAX_RATE;
  return { subtotal, tax, total: subtotal + tax };
}
```

### 3. Early Returns (Guard Clauses)

```typescript
// ❌ BAD: Deep nesting
function getDiscount(customer: Customer): number {
  if (customer) {
    if (customer.isActive) {
      if (customer.loyaltyYears > 5) {
        return 0.20;
      } else if (customer.loyaltyYears > 2) {
        return 0.10;
      } else {
        return 0.05;
      }
    }
  }
  return 0;
}

// ✅ GOOD: Early returns
function getDiscount(customer: Customer | null): number {
  if (!customer) return 0;
  if (!customer.isActive) return 0;
  
  if (customer.loyaltyYears > 5) return 0.20;
  if (customer.loyaltyYears > 2) return 0.10;
  return 0.05;
}
```

### 4. Avoid Magic Numbers

```typescript
// ❌ BAD: Magic numbers
if (password.length < 8) {}
if (attempts > 3) {}
setTimeout(callback, 86400000);

// ✅ GOOD: Named constants
const MIN_PASSWORD_LENGTH = 8;
const MAX_LOGIN_ATTEMPTS = 3;
const ONE_DAY_MS = 24 * 60 * 60 * 1000;

if (password.length < MIN_PASSWORD_LENGTH) {}
if (attempts > MAX_LOGIN_ATTEMPTS) {}
setTimeout(callback, ONE_DAY_MS);
```

### 5. DRY (Don't Repeat Yourself)

```typescript
// ❌ BAD: Repeated validation
function createUser(data: UserData) {
  if (!data.email) throw new Error('Email required');
  if (!data.email.includes('@')) throw new Error('Invalid email');
  // ...
}

function updateUser(data: UserData) {
  if (!data.email) throw new Error('Email required');
  if (!data.email.includes('@')) throw new Error('Invalid email');
  // ...
}

// ✅ GOOD: Extract common logic
function validateEmail(email: string): void {
  if (!email) throw new Error('Email required');
  if (!email.includes('@')) throw new Error('Invalid email');
}

function createUser(data: UserData) {
  validateEmail(data.email);
  // ...
}

function updateUser(data: UserData) {
  validateEmail(data.email);
  // ...
}
```

### 6. Explain Intent, Not Implementation

```typescript
// ❌ BAD: What the code does (obvious)
// Loop through array and increment counter
for (let i = 0; i < items.length; i++) {
  counter++;
}

// ❌ BAD: Commented-out code
// const oldImplementation = () => { ... };

// ✅ GOOD: Explain WHY
// Use retry because external API has intermittent 502 errors
const result = await retryWithBackoff(fetchData, 3);

// ✅ GOOD: Document gotchas
// WARNING: This returns cached data up to 5 minutes old
const user = getCachedUser(id);
```

## Refactoring Patterns

### Extract Method
```typescript
// Before: Complex inline logic
function renderInvoice(invoice: Invoice) {
  // Calculate totals
  const subtotal = invoice.items.reduce((s, i) => s + i.amount, 0);
  const tax = subtotal * 0.1;
  const total = subtotal + tax;
  
  // Format output
  return `
    Invoice #${invoice.id}
    Subtotal: $${subtotal.toFixed(2)}
    Tax: $${tax.toFixed(2)}
    Total: $${total.toFixed(2)}
  `;
}

// After: Extracted methods
function renderInvoice(invoice: Invoice) {
  const totals = calculateInvoiceTotals(invoice);
  return formatInvoice(invoice, totals);
}

function calculateInvoiceTotals(invoice: Invoice): InvoiceTotals {
  const subtotal = invoice.items.reduce((s, i) => s + i.amount, 0);
  const tax = subtotal * TAX_RATE;
  return { subtotal, tax, total: subtotal + tax };
}
```

### Replace Conditional with Polymorphism
```typescript
// Before: Switch on type
function calculateArea(shape: Shape): number {
  switch (shape.type) {
    case 'circle':
      return Math.PI * shape.radius ** 2;
    case 'rectangle':
      return shape.width * shape.height;
    case 'triangle':
      return 0.5 * shape.base * shape.height;
    default:
      throw new Error('Unknown shape');
  }
}

// After: Polymorphic behavior
interface Shape {
  calculateArea(): number;
}

class Circle implements Shape {
  constructor(private radius: number) {}
  calculateArea(): number {
    return Math.PI * this.radius ** 2;
  }
}

class Rectangle implements Shape {
  constructor(private width: number, private height: number) {}
  calculateArea(): number {
    return this.width * this.height;
  }
}
```

### Replace Nested Conditionals with Strategy
```typescript
// Before: Nested conditions
function getPrice(product: Product, user: User): number {
  let price = product.basePrice;
  
  if (user.isMember) {
    if (user.membershipLevel === 'gold') {
      price *= 0.8;
    } else if (user.membershipLevel === 'silver') {
      price *= 0.9;
    }
  }
  
  if (product.category === 'clearance') {
    price *= 0.5;
  }
  
  return price;
}

// After: Strategy pattern
const discountStrategies: Record<string, (price: number) => number> = {
  gold: (price) => price * 0.8,
  silver: (price) => price * 0.9,
  none: (price) => price,
};

function getPrice(product: Product, user: User): number {
  const memberDiscount = discountStrategies[user.membershipLevel || 'none'];
  const categoryMultiplier = product.category === 'clearance' ? 0.5 : 1;
  
  return memberDiscount(product.basePrice) * categoryMultiplier;
}
```

## Code Smells to Avoid

| Smell | Problem | Solution |
|-------|---------|----------|
| Long Function | > 50 lines | Extract methods |
| Long Parameter List | > 3 params | Use object parameter |
| Duplicate Code | Same logic repeated | Extract function |
| Dead Code | Unused code | Delete it |
| Magic Numbers | Unexplained values | Use named constants |
| Deep Nesting | > 3 levels | Early returns |
| God Class | Does everything | Split responsibilities |
| Feature Envy | Uses other class data | Move method |

## Quality Metrics

| Metric | Target |
|--------|--------|
| Function Length | < 50 lines |
| File Length | < 400 lines |
| Cyclomatic Complexity | < 10 |
| Parameters | < 4 |
| Nesting Depth | < 4 |
