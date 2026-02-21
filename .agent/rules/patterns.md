# Code Patterns Rules - ALWAYS FOLLOW

> Standard patterns for API responses, error handling, and common scenarios.

## 📡 API Response Formats

### Success Response
```typescript
// Standard success response
interface ApiResponse<T> {
  success: true;
  data: T;
  meta?: {
    page?: number;
    pageSize?: number;
    total?: number;
  };
}

// Example
{
  "success": true,
  "data": {
    "id": "123",
    "name": "John Doe"
  }
}
```

### Error Response
```typescript
interface ApiError {
  success: false;
  error: {
    code: string;        // Machine-readable code
    message: string;     // Human-readable message
    details?: unknown;   // Additional context
  };
}

// Example
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Email is required",
    "details": {
      "field": "email",
      "constraint": "required"
    }
  }
}
```

### Paginated Response
```typescript
{
  "success": true,
  "data": [...],
  "meta": {
    "page": 1,
    "pageSize": 20,
    "total": 100,
    "totalPages": 5
  }
}
```

## ⚠️ Error Handling

### Try-Catch Pattern
```typescript
// ✅ GOOD: Specific error handling
async function fetchUser(id: string) {
  try {
    const user = await db.user.findUnique({ where: { id } });
    if (!user) {
      throw new NotFoundError(`User ${id} not found`);
    }
    return user;
  } catch (error) {
    if (error instanceof NotFoundError) {
      throw error; // Re-throw known errors
    }
    // Log unexpected errors
    logger.error('Failed to fetch user', { id, error });
    throw new InternalError('Failed to fetch user');
  }
}
```

### Error Classes
```typescript
// Define custom error classes
class AppError extends Error {
  constructor(
    public code: string,
    message: string,
    public statusCode: number = 500
  ) {
    super(message);
    this.name = 'AppError';
  }
}

class NotFoundError extends AppError {
  constructor(message: string) {
    super('NOT_FOUND', message, 404);
  }
}

class ValidationError extends AppError {
  constructor(message: string, public details?: unknown) {
    super('VALIDATION_ERROR', message, 400);
  }
}
```

### Express Error Middleware
```typescript
function errorHandler(
  err: Error,
  req: Request,
  res: Response,
  next: NextFunction
) {
  if (err instanceof AppError) {
    return res.status(err.statusCode).json({
      success: false,
      error: {
        code: err.code,
        message: err.message,
      },
    });
  }

  // Unknown error - don't leak details
  logger.error('Unhandled error', { err });
  return res.status(500).json({
    success: false,
    error: {
      code: 'INTERNAL_ERROR',
      message: 'An unexpected error occurred',
    },
  });
}
```

## 🔄 Async Patterns

### Promise.all for Parallel Operations
```typescript
// ✅ GOOD: Parallel execution
const [users, products, orders] = await Promise.all([
  fetchUsers(),
  fetchProducts(),
  fetchOrders(),
]);

// ❌ BAD: Sequential when parallel is possible
const users = await fetchUsers();
const products = await fetchProducts();
const orders = await fetchOrders();
```

### Retry with Backoff
```typescript
async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries: number = 3,
  baseDelay: number = 1000
): Promise<T> {
  let lastError: Error;
  
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as Error;
      const delay = baseDelay * Math.pow(2, attempt);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  
  throw lastError!;
}
```

### Timeout Wrapper
```typescript
async function withTimeout<T>(
  promise: Promise<T>,
  ms: number
): Promise<T> {
  const timeout = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new Error('Timeout')), ms)
  );
  return Promise.race([promise, timeout]);
}

// Usage
const data = await withTimeout(fetchData(), 5000);
```

## 🏗️ Repository Pattern

```typescript
interface Repository<T> {
  findById(id: string): Promise<T | null>;
  findAll(options?: QueryOptions): Promise<T[]>;
  create(data: Partial<T>): Promise<T>;
  update(id: string, data: Partial<T>): Promise<T>;
  delete(id: string): Promise<void>;
}

class UserRepository implements Repository<User> {
  constructor(private db: Database) {}

  async findById(id: string): Promise<User | null> {
    return this.db.user.findUnique({ where: { id } });
  }

  async findAll(options?: QueryOptions): Promise<User[]> {
    return this.db.user.findMany({
      skip: options?.offset,
      take: options?.limit,
      where: options?.filter,
    });
  }
  
  // ... other methods
}
```

## 🎣 React Patterns

### Custom Hook Pattern
```typescript
function useFetch<T>(url: string) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    
    async function fetch() {
      try {
        setLoading(true);
        const response = await api.get<T>(url);
        if (!cancelled) {
          setData(response.data);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err as Error);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }
    
    fetch();
    return () => { cancelled = true; };
  }, [url]);

  return { data, loading, error };
}
```

### Compound Component Pattern
```typescript
// Parent provides context
const TabsContext = createContext<TabsContextValue | null>(null);

function Tabs({ children, defaultTab }: TabsProps) {
  const [activeTab, setActiveTab] = useState(defaultTab);
  return (
    <TabsContext.Provider value={{ activeTab, setActiveTab }}>
      {children}
    </TabsContext.Provider>
  );
}

// Children consume context
Tabs.List = function TabList({ children }) {
  return <div className="tab-list">{children}</div>;
};

Tabs.Tab = function Tab({ id, children }) {
  const { activeTab, setActiveTab } = useContext(TabsContext);
  return (
    <button 
      className={activeTab === id ? 'active' : ''} 
      onClick={() => setActiveTab(id)}
    >
      {children}
    </button>
  );
};

// Usage
<Tabs defaultTab="tab1">
  <Tabs.List>
    <Tabs.Tab id="tab1">Tab 1</Tabs.Tab>
    <Tabs.Tab id="tab2">Tab 2</Tabs.Tab>
  </Tabs.List>
</Tabs>
```

## 🔐 Authentication Pattern

```typescript
// Middleware pattern
function authMiddleware(
  req: Request,
  res: Response,
  next: NextFunction
) {
  const token = req.headers.authorization?.split(' ')[1];
  
  if (!token) {
    return res.status(401).json({
      success: false,
      error: { code: 'UNAUTHORIZED', message: 'No token provided' },
    });
  }

  try {
    const decoded = jwt.verify(token, process.env.JWT_SECRET!);
    req.user = decoded;
    next();
  } catch {
    return res.status(401).json({
      success: false,
      error: { code: 'INVALID_TOKEN', message: 'Invalid token' },
    });
  }
}
```
