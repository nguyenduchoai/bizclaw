# Testing Rules - ALWAYS FOLLOW

> Test-Driven Development (TDD) methodology with comprehensive coverage.

## 🎯 Coverage Requirements

### Minimum Standards
```
Overall Coverage:    80% minimum
Critical Paths:      100% coverage
Edge Cases:          All documented
Error Scenarios:     All tested
```

### Coverage Types
| Type | Scope | Target |
|------|-------|--------|
| Unit | Functions, utilities | 90% |
| Integration | API, database | 80% |
| E2E | Critical user flows | Key paths |

## 🔴🟢 TDD Workflow (Red-Green-Refactor)

### Step 1: RED - Write Failing Test
```typescript
// Write test FIRST
describe('calculateDiscount', () => {
  it('should apply 10% discount for orders over $100', () => {
    const result = calculateDiscount(150);
    expect(result).toBe(135); // 10% off
  });
});
```

### Step 2: GREEN - Minimal Implementation
```typescript
// Implement JUST ENOUGH to pass
function calculateDiscount(amount: number): number {
  if (amount > 100) {
    return amount * 0.9;
  }
  return amount;
}
```

### Step 3: REFACTOR - Improve Code
```typescript
// Clean up while tests pass
const DISCOUNT_THRESHOLD = 100;
const DISCOUNT_RATE = 0.1;

function calculateDiscount(amount: number): number {
  if (amount <= DISCOUNT_THRESHOLD) return amount;
  return amount * (1 - DISCOUNT_RATE);
}
```

## 📋 Test Structure

### Arrange-Act-Assert (AAA)
```typescript
it('should format date correctly', () => {
  // Arrange
  const date = new Date('2026-01-21');
  
  // Act
  const result = formatDate(date);
  
  // Assert
  expect(result).toBe('21/01/2026');
});
```

### Test Naming
```typescript
// Pattern: should [expected behavior] when [condition]
it('should return empty array when input is null')
it('should throw error when user is not authenticated')
it('should retry 3 times when API call fails')
```

## 🧪 Test Types

### Unit Tests (Jest/Vitest)
```typescript
describe('UserService', () => {
  describe('validateEmail', () => {
    it('should return true for valid email', () => {
      expect(validateEmail('user@example.com')).toBe(true);
    });
    
    it('should return false for invalid email', () => {
      expect(validateEmail('invalid')).toBe(false);
    });
    
    it('should handle edge cases', () => {
      expect(validateEmail('')).toBe(false);
      expect(validateEmail(null)).toBe(false);
    });
  });
});
```

### Integration Tests
```typescript
describe('POST /api/users', () => {
  it('should create user and return 201', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ name: 'John', email: 'john@example.com' });
    
    expect(response.status).toBe(201);
    expect(response.body).toHaveProperty('id');
  });
  
  it('should return 400 for invalid data', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ name: '' });
    
    expect(response.status).toBe(400);
  });
});
```

### E2E Tests (Playwright)
```typescript
test('user can complete checkout flow', async ({ page }) => {
  // Login
  await page.goto('/login');
  await page.fill('[data-testid="email"]', 'user@example.com');
  await page.fill('[data-testid="password"]', 'password');
  await page.click('[data-testid="submit"]');
  
  // Add to cart
  await page.goto('/products/1');
  await page.click('[data-testid="add-to-cart"]');
  
  // Checkout
  await page.goto('/checkout');
  await page.click('[data-testid="place-order"]');
  
  // Verify
  await expect(page.locator('[data-testid="order-confirmation"]')).toBeVisible();
});
```

## ❌ Testing Anti-Patterns

### DON'T Test Implementation Details
```typescript
// ❌ WRONG: Testing internal state
it('should set isLoading to true', () => {
  const { result } = renderHook(() => useFetch('/api/data'));
  expect(result.current.isLoading).toBe(true);
});

// ✅ CORRECT: Test user-visible behavior
it('should show loading spinner while fetching', async () => {
  render(<DataComponent />);
  expect(screen.getByRole('progressbar')).toBeInTheDocument();
});
```

### DON'T Use Brittle Selectors
```typescript
// ❌ WRONG: Brittle CSS selectors
await page.click('.btn-primary.submit-form');
await page.locator('div > div > button').click();

// ✅ CORRECT: Semantic selectors
await page.click('[data-testid="submit-button"]');
await page.getByRole('button', { name: 'Submit' }).click();
```

### DON'T Create Dependent Tests
```typescript
// ❌ WRONG: Tests depend on each other
it('should create user', () => { userId = createUser(); });
it('should get user', () => { getUser(userId); }); // Depends on previous test!

// ✅ CORRECT: Independent tests
it('should create user', () => {
  const userId = createUser();
  expect(userId).toBeDefined();
});

it('should get user', () => {
  const userId = createUser(); // Setup within test
  const user = getUser(userId);
  expect(user).toBeDefined();
});
```

## 🔧 Test Configuration

### vitest.config.ts
```typescript
export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
    },
  },
});
```

### CI/CD Integration
```yaml
# .github/workflows/test.yml
- name: Run Tests
  run: npm test -- --coverage
  
- name: Check Coverage
  run: |
    coverage=$(cat coverage/coverage-summary.json | jq '.total.lines.pct')
    if (( $(echo "$coverage < 80" | bc -l) )); then
      echo "Coverage $coverage% is below 80%"
      exit 1
    fi
```

## ⚠️ Enforcement

- Tests MUST pass before merging
- Coverage MUST meet thresholds
- New features REQUIRE tests
- Bug fixes REQUIRE regression tests
