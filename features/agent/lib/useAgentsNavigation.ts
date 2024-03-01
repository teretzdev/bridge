// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

import { useRouteQuery } from '@vueuse/router'

export const useAgentsNavigation = () => {
  const isCreateAgent = useRouteQuery('create', 'false', { transform: (value: 'false' | 'true') => value === 'true' })
  const enableCreateAgent = () => {
    isCreateAgent.value = true
    selectedAgent.value = null
  }
  const disableCreateAgent = () => {
    isCreateAgent.value = false
  }

  const selectedAgent = useRouteQuery('agent', '', {
    transform: (value: string) => (isNaN(Number(value)) ? null : Number(value)),
  })
  const setSelectedAgent = (id: Nullable<number>) => {
    selectedAgent.value = id
  }
  return {
    isCreateAgent: readonly(isCreateAgent),
    enableCreateAgent,
    disableCreateAgent,
    selectedAgent: readonly(selectedAgent),
    setSelectedAgent,
  }
}