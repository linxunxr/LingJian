import { beforeEach, describe, expect, it, vi } from 'vitest'

const { actOnIssue } = vi.hoisted(() => ({ actOnIssue: vi.fn() }))
vi.mock('@/composables/useIssues', () => ({
  useIssues: () => ({ actOnIssue }),
}))

import { mount } from '@vue/test-utils'

import CommentDialog from './CommentDialog.vue'

beforeEach(() => {
  actOnIssue.mockReset()
  actOnIssue.mockResolvedValue(true)
})

describe('CommentDialog', () => {
  it('标题携带 issue 编号，空文本禁用提交', () => {
    const wrapper = mount(CommentDialog, { props: { issueNumber: 42 } })

    expect(wrapper.find('.dialog-title').text()).toBe('添加评论 · #42')
    const submit = wrapper.findAll('button').find((b) => b.text() === '提交')!
    expect(submit.attributes('disabled')).toBeDefined()
  })

  it('提交成功后关闭', async () => {
    const wrapper = mount(CommentDialog, { props: { issueNumber: 42 } })

    await wrapper.find('textarea').setValue('看起来已修复')
    const submit = wrapper.findAll('button').find((b) => b.text() === '提交')!
    await submit.trigger('click')

    expect(actOnIssue).toHaveBeenCalledWith(42, 'comment', { body: '看起来已修复' })
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('提交失败保留弹窗供重试', async () => {
    actOnIssue.mockResolvedValue(false)
    const wrapper = mount(CommentDialog, { props: { issueNumber: 42 } })

    await wrapper.find('textarea').setValue('评论')
    const submit = wrapper.findAll('button').find((b) => b.text() === '提交')!
    await submit.trigger('click')

    expect(wrapper.emitted('close')).toBeUndefined()
  })
})
