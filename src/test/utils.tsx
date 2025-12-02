import { render, RenderOptions } from '@testing-library/react'
import { ReactElement, ReactNode } from 'react'
import { vi } from 'vitest'

// Custom render function for testing
const AllTheProviders = ({ children }: { children: ReactNode }) => {
  return <>{children}</>
}

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) => render(ui, { wrapper: AllTheProviders, ...options })

// Re-export everything from React Testing Library
export * from '@testing-library/react'
export { customRender as render }

// Mock utilities
export const createMockTauriCommand = (commandName: string) => {
  return vi.mocked(window.__TAURI__.invoke).mockResolvedValue({})
}

export const createMockEventEmitter = () => {
  const listeners = new Map()

  return {
    on: vi.fn((event: string, callback: Function) => {
      if (!listeners.has(event)) {
        listeners.set(event, [])
      }
      listeners.get(event).push(callback)
    }),
    off: vi.fn((event: string, callback: Function) => {
      if (listeners.has(event)) {
        const callbacks = listeners.get(event)
        const index = callbacks.indexOf(callback)
        if (index > -1) {
          callbacks.splice(index, 1)
        }
      }
    }),
    emit: vi.fn((event: string, data?: any) => {
      if (listeners.has(event)) {
        listeners.get(event).forEach((callback: Function) => callback(data))
      }
    }),
  }
}