import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

import { flushPromises, mount } from '@vue/test-utils'

import McpCard from './McpCard.vue'

const mockedInvoke = vi.mocked(invoke)

function statusOf(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    enabled: true,
    running: true,
    port: 3920,
    listeningUrl: 'http://127.0.0.1:3920/mcp',
    allowWrite: false,
    lastError: null,
    ...overrides,
  }
}

beforeEach(() => {
  mockedInvoke.mockReset()
})

async function mountCard(overrides: Record<string, unknown> = {}) {
  mockedInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'mcp_status') return statusOf(overrides)
    if (cmd === 'mcp_set_config') return statusOf(overrides)
    throw new Error(`unexpected command: ${cmd}`)
  })
  const wrapper = mount(McpCard)
  await flushPromises()
  return wrapper
}

describe('McpCard', () => {
  it('挂载时拉取运行状态并回填开关与端口', async () => {
    const wrapper = await mountCard({ allowWrite: true } as never)

    expect(mockedInvoke).toHaveBeenCalledWith('mcp_status')
    const boxes = wrapper.findAll('input[type="checkbox"]')
    expect((boxes[0].element as HTMLInputElement).checked).toBe(true)
    expect((boxes[1].element as HTMLInputElement).checked).toBe(true)
    expect((wrapper.find('.port-input').element as HTMLInputElement).value).toBe('3920')
  })

  it('勾选开关立即保存生效，无需点击应用配置', async () => {
    const wrapper = await mountCard()

    const boxes = wrapper.findAll('input[type="checkbox"]')
    await boxes[1].setValue(true)

    expect(mockedInvoke).toHaveBeenCalledWith('mcp_set_config', {
      enabled: true,
      port: 3920,
      allowWrite: true,
    })
  })

  it('开关保存用已持久化端口，不受输入框半改值影响', async () => {
    const wrapper = await mountCard()

    await wrapper.find('.port-input').setValue(5000)
    await wrapper.findAll('input[type="checkbox"]')[0].setValue(false)

    expect(mockedInvoke).toHaveBeenCalledWith('mcp_set_config', {
      enabled: false,
      port: 3920,
      allowWrite: false,
    })
  })

  it('应用配置按钮提交输入框中的端口', async () => {
    const wrapper = await mountCard()

    await wrapper.find('.port-input').setValue(5000)
    const apply = wrapper.findAll('button').find((b) => b.text() === '应用配置')!
    await apply.trigger('click')
    await flushPromises()

    expect(mockedInvoke).toHaveBeenCalledWith('mcp_set_config', {
      enabled: true,
      port: 5000,
      allowWrite: false,
    })
  })

  it('保存失败时报错并回读磁盘状态回填表单', async () => {
    const wrapper = await mountCard()

    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'mcp_status') return statusOf({ allowWrite: false })
      if (cmd === 'mcp_set_config') throw new Error('MCP 端口 3920 监听失败：地址已被占用')
      throw new Error(`unexpected command: ${cmd}`)
    })

    await wrapper.findAll('input[type="checkbox"]')[1].setValue(true)
    await flushPromises()

    expect(wrapper.find('.message.error').exists()).toBe(true)
    // 回填后勾选态与磁盘一致，不保留假勾选
    expect(
      (wrapper.findAll('input[type="checkbox"]')[1].element as HTMLInputElement).checked,
    ).toBe(false)
  })

  it('enabled 但未运行时展示启动失败原因', async () => {
    const wrapper = await mountCard({
      running: false,
      lastError: 'MCP 端口 3920 监听失败：地址已被占用',
    } as never)

    const msgs = wrapper.findAll('.message.error')
    expect(msgs[msgs.length - 1].text()).toContain('MCP 端口 3920 监听失败')
  })
})
