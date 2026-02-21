# Performance Rules - ALWAYS FOLLOW

> Optimize AI model usage, context window management, and code performance.

## 🤖 AI Model Selection Strategy

### Model Tiers
| Model | Use Case | Cost | Speed |
|-------|----------|------|-------|
| **Haiku 4.5** | Lightweight tasks, frequent calls | Low | Fast |
| **Sonnet 4.5** | Main development, coding | Medium | Balanced |
| **Opus 4.5** | Complex architecture, deep reasoning | High | Slow |

### When to Use Each

**Haiku 4.5** (90% of Sonnet capability):
- Simple code generation
- Pair programming
- Worker agents in multi-agent systems
- Repetitive tasks
- Quick lookups and formatting

**Sonnet 4.5** (Best coding model):
- Main development work
- Feature implementation
- Debugging complex issues
- Orchestrating workflows

**Opus 4.5** (Maximum reasoning):
- Architectural decisions
- Complex problem decomposition
- Research and analysis
- Critical security reviews

## 📊 Context Window Management

### The 200K Problem
```
Full context:     200,000 tokens
With 20 MCPs:     ~70,000 tokens (tools consume context!)
Effective usage:  ~50,000 tokens for conversation
```

### Rules
| Rule | Why |
|------|-----|
| Keep MCPs < 10 enabled | Each MCP consumes ~5-10K tokens |
| Keep tools < 80 active | Tool definitions eat context |
| Use `disabledMcpServers` | Disable per-project unused MCPs |
| Compact at 70% usage | Performance degrades at high usage |

### Avoid Last 20% of Context For
- Large refactoring operations
- Multi-file feature implementations
- Complex debugging sessions
- Anything requiring full context recall

### Lower Context Sensitivity Tasks
- Single-file edits
- Independent utility creation
- Documentation updates
- Simple bug fixes

## ⚡ Ultrathink + Plan Mode

For complex tasks requiring deep reasoning:

```
1. Enable "Ultrathink" for enhanced thinking
2. Enable "Plan Mode" for structured approach
3. "Rev the engine" - Multiple critique rounds
4. Use split-role sub-agents for diverse analysis
```

### When to Use
- System architecture design
- Security vulnerability analysis
- Performance optimization strategy
- Complex algorithm design

## 🔧 Code Performance

### React Performance
```typescript
// ✅ GOOD: Memoization
const MemoizedComponent = React.memo(ExpensiveComponent);
const memoizedValue = useMemo(() => compute(data), [data]);
const memoizedCallback = useCallback(() => action(), [deps]);

// ✅ GOOD: Code splitting
const LazyComponent = React.lazy(() => import('./Heavy'));

// ✅ GOOD: Virtual lists for large data
import { VirtualList } from 'react-virtual';
```

### Database Performance
```sql
-- ✅ GOOD: Add indexes for frequently queried columns
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_orders_user_date ON orders(user_id, created_at);

-- ✅ GOOD: Select only needed columns
SELECT id, name FROM users WHERE status = 'active';

-- ❌ BAD: Select all
SELECT * FROM users;
```

### API Performance
```typescript
// ✅ GOOD: Pagination
const users = await db.user.findMany({
  take: 20,
  skip: page * 20,
});

// ✅ GOOD: Caching
const cachedData = await redis.get(`user:${id}`);
if (cachedData) return JSON.parse(cachedData);

// ✅ GOOD: Rate limiting
const limiter = rateLimit({ windowMs: 60000, max: 100 });
```

## 🏗️ Build Performance

### Bundle Optimization
```typescript
// vite.config.ts
export default {
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          utils: ['lodash', 'date-fns'],
        },
      },
    },
  },
};
```

### Image Optimization
```typescript
// Use next/image or sharp
import Image from 'next/image';
<Image src={src} width={800} height={600} quality={75} />

// Or convert to WebP
sharp(input).webp({ quality: 80 }).toFile(output);
```

## 📈 Monitoring

### Core Web Vitals Targets
| Metric | Good | Needs Improvement |
|--------|------|-------------------|
| LCP | < 2.5s | < 4.0s |
| FID | < 100ms | < 300ms |
| CLS | < 0.1 | < 0.25 |

### Profiling Commands
```bash
# Lighthouse audit
npx lighthouse https://yoursite.com --view

# Bundle analysis
npx vite-bundle-visualizer

# React DevTools Profiler
# Use Chrome DevTools → React → Profiler
```

## ⚠️ Performance Anti-Patterns

### Avoid
- N+1 queries (use `include` or `join`)
- Synchronous file operations in request handlers
- Unindexed database queries on large tables
- Large images without lazy loading
- Blocking the main thread with heavy computation
- Loading all data upfront (use pagination)
- Re-rendering on every state change
