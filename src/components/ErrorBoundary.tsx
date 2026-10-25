import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * 全局错误边界：任何未捕获的渲染/Effect 异常都不应让窗口永久黑屏。
 * 兜底 UI 提供重载入口，用户不必杀进程。
 *
 * 兜底文案刻意写成中英双语硬编码，而不是走 i18n —— 出错时坏掉的可能正是
 * i18n 本身，那时再依赖它取文案就一个字也显示不出来。
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Uncaught error in renderer:', error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      /* 视觉走 .error-fallback 类（app.css）：内联样式是一笔设计债 ——
         兜底 UI 也该吃同一套令牌，主题切换时才不会脱队（2026-10-25 审计 D-78 债务项） */
      return (
        <div className="error-fallback">
          <h1 className="error-fallback-title">出错了 / Something went wrong</h1>
          <pre className="error-fallback-msg">
            {String(this.state.error?.message || this.state.error)}
          </pre>
          <button className="btn btn-primary" onClick={() => window.location.reload()}>
            重新加载 / Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
