import { beforeEach, describe, expect, it, vi } from 'vitest'

const { actOnIssue } = vi.hoisted(() => ({ actOnIssue: vi.fn() }))
vi.mock('@/composables/useIssues', () => ({
  useIssues: () => ({ actOnIssue }),
}))

import { mount } from '@vue/test-utils'

import CloseIssueDialog from './CloseIssueDialog.vue'

beforeEach(() => {
  actOnIssue.mockReset()
  actOnIssue.mockResolvedValue(true)
})

async function confirm(wrapper: ReturnType<typeof mount>) {
  await wrapper.find('.version-input').setValue('0.9.19')
  const btn = wrapper.findAll('button').find((b) => b.text() === '确认关闭')!
  await btn.trigger('click')
}

describe('CloseIssueDialog', () => {
  it('空版本号禁用确认按钮', () => {
    const wrapper = mount(CloseIssueDialog, { props: { issueNumber: 42, labels: [] } })

    const btn = wrapper.findAll('button').find((b) => b.text() === '确认关闭')!
    expect(btn.attributes('disabled')).toBeDefined()
  })

  it('确认后串行执行 close → 追加版本标签 → 解决评论，然后关闭', async () => {
    const wrapper = mount(CloseIssueDialog, { props: { issueNumber: 42, labels: ['高优先级'] } })

    await confirm(wrapper)

    expect(actOnIssue.mock.calls.map((c) => [c[0], c[1]])).toEqual([
      [42, 'close'],
      [42, 'setLabels'],
      [42, 'comment'],
    ])
    // 标签 = 原有标签去重合并 v0.9.19
    expect(actOnIssue).toHaveBeenNthCalledWith(2, 42, 'setLabels', {
      labels: ['高优先级', 'v0.9.19'],
    })
    expect(actOnIssue).toHaveBeenNthCalledWith(3, 42, 'comment', {
      body: '已在挂机仙途 v0.9.19 中标记为已处理',
    })
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('close 失败保留弹窗且不执行后续步骤', async () => {
    actOnIssue.mockImplementation(async (_n: number, action: string) => action !== 'close')
    const wrapper = mount(CloseIssueDialog, { props: { issueNumber: 42, labels: [] } })

    await confirm(wrapper)

    expect(actOnIssue).toHaveBeenCalledTimes(1)
    expect(wrapper.emitted('close')).toBeUndefined()
  })

  it('标签/评论失败不阻断关闭弹窗（Issue 已关闭是主目标）', async () => {
    actOnIssue.mockImplementation(async (_n: number, action: string) => action === 'close')
    const wrapper = mount(CloseIssueDialog, { props: { issueNumber: 42, labels: [] } })

    await confirm(wrapper)

    expect(actOnIssue).toHaveBeenCalledTimes(3)
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('版本号带 v 前缀时归一化', async () => {
    const wrapper = mount(CloseIssueDialog, { props: { issueNumber: 42, labels: [] } })

    await wrapper.find('.version-input').setValue('v0.9.20')
    const btn = wrapper.findAll('button').find((b) => b.text() === '确认关闭')!
    await btn.trigger('click')

    expect(actOnIssue).toHaveBeenNthCalledWith(2, 42, 'setLabels', { labels: ['v0.9.20'] })
  })
})
