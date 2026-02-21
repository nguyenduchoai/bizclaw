# Coding Style Rules - ALWAYS FOLLOW

> Consistent, maintainable code across all projects.

## 📏 File Organization

### Size Limits
```
- Max 400 lines per file (strongly preferred)
- Max 600 lines absolute maximum
- Split larger files into modules
```

### Structure Order
```typescript
// 1. Imports (external → internal → relative)
import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { formatDate } from '@/lib/utils';
import { Button } from './Button';

// 2. Types/Interfaces
interface Props {
  name: string;
}

// 3. Constants
const MAX_ITEMS = 100;

// 4. Component/Function
export function Component({ name }: Props) {
  // ...
}

// 5. Helpers (private, at bottom)
function helperFunction() {
  // ...
}
```

## 🔒 Immutability First

### PREFER Immutable Operations
```typescript
// ✅ CORRECT: Spread operator
const newArray = [...oldArray, newItem];
const newObject = { ...oldObject, newKey: value };

// ✅ CORRECT: Array methods that return new arrays
const filtered = items.filter(x => x.active);
const mapped = items.map(x => ({ ...x, processed: true }));

// ❌ AVOID: Mutation
oldArray.push(newItem);
oldObject.newKey = value;
```

### State Updates
```typescript
// ✅ CORRECT: Functional updates
setState(prev => [...prev, newItem]);
setObject(prev => ({ ...prev, key: value }));

// ❌ WRONG: Direct mutation
state.push(newItem); // BUG: Won't trigger re-render
```

## 📝 Naming Conventions

### Files
```
components/       PascalCase.tsx      (Button.tsx)
hooks/            camelCase.ts        (useAuth.ts)
utils/            camelCase.ts        (formatDate.ts)
types/            camelCase.ts        (userTypes.ts)
constants/        SCREAMING_SNAKE.ts  (API_ROUTES.ts)
```

### Variables & Functions
```typescript
// Variables: camelCase, descriptive
const userProfile = {};
const isAuthenticated = true;
const hasPermission = false;

// Functions: camelCase, verb-first
function getUserById(id: string) {}
function calculateTotal(items: Item[]) {}
async function fetchUserData() {}

// Boolean: is/has/should/can prefix
const isLoading = true;
const hasError = false;
const shouldRefresh = true;
const canEdit = user.role === 'admin';
```

### Types & Interfaces
```typescript
// Types: PascalCase, descriptive
type UserRole = 'admin' | 'user' | 'guest';
type ApiResponse<T> = { data: T; error?: string };

// Interfaces: PascalCase, noun
interface User {
  id: string;
  name: string;
}

// Props: ComponentNameProps
interface ButtonProps {
  label: string;
  onClick: () => void;
}
```

## 🧹 Clean Code Principles

### Single Responsibility
```typescript
// ❌ WRONG: Function does too much
function processUser(user) {
  validateUser(user);
  saveToDatabase(user);
  sendEmail(user);
  updateAnalytics(user);
}

// ✅ CORRECT: Single responsibility
function validateUser(user) { /* only validation */ }
function saveUser(user) { /* only persistence */ }
function notifyUser(user) { /* only notification */ }
```

### Early Returns
```typescript
// ❌ AVOID: Nested conditions
function process(data) {
  if (data) {
    if (data.isValid) {
      if (data.items.length > 0) {
        // actual logic buried deep
      }
    }
  }
}

// ✅ PREFER: Early returns (guard clauses)
function process(data) {
  if (!data) return null;
  if (!data.isValid) return null;
  if (data.items.length === 0) return [];
  
  // actual logic at top level
  return data.items.map(processItem);
}
```

### Avoid Magic Numbers
```typescript
// ❌ WRONG: Magic numbers
if (items.length > 10) { /* what is 10? */ }
setTimeout(callback, 3000);

// ✅ CORRECT: Named constants
const MAX_VISIBLE_ITEMS = 10;
const DEBOUNCE_DELAY_MS = 3000;

if (items.length > MAX_VISIBLE_ITEMS) {}
setTimeout(callback, DEBOUNCE_DELAY_MS);
```

## 💬 Comments

### When to Comment
```typescript
// ✅ GOOD: Explain WHY, not WHAT
// Using retry logic because external API has intermittent failures
const result = await retryWithBackoff(fetchData, 3);

// ✅ GOOD: Document complex algorithms
// Fisher-Yates shuffle - O(n) time, O(1) space
function shuffle<T>(array: T[]): T[] { /* ... */ }

// ✅ GOOD: Warn about gotchas
// WARNING: This endpoint returns cached data for up to 5 minutes
```

### When NOT to Comment
```typescript
// ❌ BAD: Obvious comments
// Increment counter
counter++;

// ❌ BAD: Commented-out code (just delete it)
// const oldImplementation = () => { ... };
```

## 📊 Code Quality Metrics

| Metric | Target |
|--------|--------|
| File size | < 400 lines |
| Function length | < 50 lines |
| Cyclomatic complexity | < 10 |
| Nesting depth | < 4 levels |
| Parameters per function | < 5 |
