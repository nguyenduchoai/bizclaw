---
name: coding-standards
description: Comprehensive coding standards for TypeScript/JavaScript projects. Use when writing code, reviewing code, or establishing project conventions.
---

# Coding Standards Skill

> Consistent, maintainable, professional code across all projects.

## When to Activate

- Starting new projects
- Writing new code
- Code review sessions
- Establishing team conventions
- Onboarding new team members

## TypeScript Guidelines

### Type Annotations

```typescript
// ✅ GOOD: Explicit return types for functions
function calculateTotal(items: Item[]): number {
  return items.reduce((sum, item) => sum + item.price, 0);
}

// ✅ GOOD: Interface for object shapes
interface User {
  id: string;
  name: string;
  email: string;
  createdAt: Date;
}

// ✅ GOOD: Type for unions/aliases
type Status = 'pending' | 'active' | 'completed';
type ID = string | number;

// ❌ AVOID: any type
function process(data: any) {} // Use unknown or proper type

// ❌ AVOID: implicit any
function add(x, y) {} // Add type annotations
```

### Type Safety

```typescript
// ✅ GOOD: Null handling
function getUser(id: string): User | null {
  const user = db.find(id);
  return user ?? null;
}

// ✅ GOOD: Type guards
function isUser(obj: unknown): obj is User {
  return typeof obj === 'object' && obj !== null && 'email' in obj;
}

// ✅ GOOD: Const assertions for immutability
const ROLES = ['admin', 'user', 'guest'] as const;
type Role = typeof ROLES[number]; // 'admin' | 'user' | 'guest'

// ✅ GOOD: Generics for reusability
function first<T>(arr: T[]): T | undefined {
  return arr[0];
}
```

### Strict Mode Configuration

```json
// tsconfig.json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true
  }
}
```

## JavaScript Patterns

### Destructuring
```typescript
// ✅ GOOD: Destructure function parameters
function createUser({ name, email, role = 'user' }: CreateUserInput) {
  return { name, email, role };
}

// ✅ GOOD: Destructure arrays
const [first, second, ...rest] = items;

// ✅ GOOD: Rename during destructure
const { name: userName, email: userEmail } = user;
```

### Spread Operator
```typescript
// ✅ GOOD: Immutable object updates
const updated = { ...user, name: 'New Name' };

// ✅ GOOD: Array concatenation
const allItems = [...items1, ...items2];

// ✅ GOOD: Function arguments
Math.max(...numbers);
```

### Optional Chaining & Nullish Coalescing
```typescript
// ✅ GOOD: Safe property access
const city = user?.address?.city;

// ✅ GOOD: Default values (nullish only)
const name = user.name ?? 'Unknown';

// ✅ GOOD: Method calls
user?.getFullName?.();
```

## React Guidelines

### Component Structure
```typescript
// 1. Imports (external → internal → relative)
import React, { useState, useEffect } from 'react';
import { Button } from '@/components/ui';
import { formatDate } from './utils';

// 2. Types
interface Props {
  userId: string;
  onSelect?: (user: User) => void;
}

// 3. Component
export function UserCard({ userId, onSelect }: Props) {
  // Hooks first
  const [user, setUser] = useState<User | null>(null);
  
  // Effects
  useEffect(() => {
    fetchUser(userId).then(setUser);
  }, [userId]);
  
  // Handlers
  const handleClick = () => {
    if (user && onSelect) {
      onSelect(user);
    }
  };
  
  // Early returns
  if (!user) return <Loading />;
  
  // Render
  return (
    <div onClick={handleClick}>
      {user.name}
    </div>
  );
}

// 4. Helpers (at bottom, not exported)
function formatUserName(user: User): string {
  return `${user.firstName} ${user.lastName}`;
}
```

### Hook Rules
```typescript
// ✅ GOOD: Hooks at top level
function Component() {
  const [count, setCount] = useState(0);
  const memoized = useMemo(() => compute(count), [count]);
  
  // ...
}

// ❌ BAD: Hooks in conditions
function Component() {
  if (condition) {
    const [value, setValue] = useState(0); // ERROR!
  }
}

// ❌ BAD: Hooks in loops
function Component() {
  items.forEach(() => {
    const [item, setItem] = useState(null); // ERROR!
  });
}
```

### Custom Hooks
```typescript
// ✅ GOOD: Extract reusable logic
function useLocalStorage<T>(key: string, initialValue: T) {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = window.localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = (value: T | ((val: T) => T)) => {
    try {
      const valueToStore = value instanceof Function 
        ? value(storedValue) 
        : value;
      setStoredValue(valueToStore);
      window.localStorage.setItem(key, JSON.stringify(valueToStore));
    } catch (error) {
      console.error(error);
    }
  };

  return [storedValue, setValue] as const;
}
```

## Error Handling

### Async/Await Pattern
```typescript
// ✅ GOOD: Try-catch with proper typing
async function fetchUser(id: string): Promise<User> {
  try {
    const response = await api.get(`/users/${id}`);
    return response.data;
  } catch (error) {
    if (error instanceof NotFoundError) {
      throw new Error(`User ${id} not found`);
    }
    throw error;
  }
}
```

### Result Pattern
```typescript
// ✅ GOOD: Explicit success/failure
type Result<T, E = Error> = 
  | { success: true; data: T }
  | { success: false; error: E };

async function fetchUser(id: string): Promise<Result<User>> {
  try {
    const user = await db.user.findUnique({ where: { id } });
    if (!user) {
      return { success: false, error: new Error('Not found') };
    }
    return { success: true, data: user };
  } catch (error) {
    return { success: false, error: error as Error };
  }
}

// Usage
const result = await fetchUser('123');
if (result.success) {
  console.log(result.data.name);
} else {
  console.error(result.error.message);
}
```

## File Organization

### Directory Structure
```
src/
├── components/          # React components
│   ├── ui/              # Reusable UI components
│   └── features/        # Feature-specific components
├── hooks/               # Custom React hooks
├── lib/                 # Utilities and helpers
├── services/            # API and external services
├── types/               # TypeScript type definitions
├── constants/           # Constants and config
└── app/ or pages/       # Routes/pages
```

### Naming Conventions
| Type | Convention | Example |
|------|------------|---------|
| Components | PascalCase | `UserCard.tsx` |
| Hooks | camelCase with use | `useAuth.ts` |
| Utilities | camelCase | `formatDate.ts` |
| Types | PascalCase | `User.ts` |
| Constants | SCREAMING_SNAKE | `API_ROUTES.ts` |

## Code Quality Metrics

| Metric | Target | Description |
|--------|--------|-------------|
| File Size | < 400 lines | Split larger files |
| Function Length | < 50 lines | Extract helper functions |
| Cyclomatic Complexity | < 10 | Simplify conditionals |
| Nesting Depth | < 4 levels | Use early returns |
| Parameters | < 5 | Use object parameter |
