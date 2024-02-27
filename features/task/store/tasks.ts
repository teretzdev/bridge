// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

import { type Task } from '~/entities/tasks'
import {
  listRootTasks as listRootTasksReq,
  listChildTasks as listChildTasksReq,
  createTask as createTaskReq,
  getTask as getTaskReq,
  deleteTask as deleteTaskReq,
} from '../api'
import { groupTasks } from '../lib'
import { type CreateTask, type ListTasksParams } from '../model'

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([])
  const getById = (id: number | string | undefined): Task | undefined => {
    if (id === undefined) {
      return undefined
    }

    if (typeof id === 'string') {
      id = parseInt(id, 10)
    }

    return tasks.value.find((a) => a.id === id)
  }
  const tasksGroupsByStatus = computed(() => {
    return groupTasks(tasks.value)
  })

  const listRootTasks = async (params: ListTasksParams) => {
    const rootTasks = await listRootTasksReq(params)
    rootTasks.forEach((task) => {
      if (!tasks.value.find((a) => a.id === task.id)) {
        tasks.value.push(task)
      }
    })
  }

  const listChildTasks = async (id: number) => {
    const childTasks = await listChildTasksReq(id)
    childTasks.forEach((task) => {
      if (!tasks.value.find((a) => a.id === task.id)) {
        tasks.value.push(task)
      }
    })
  }

  const createTask = async (task: CreateTask) => {
    const newTask = await createTaskReq(task)
    tasks.value.push(newTask)
  }

  const getTask = async (id: number) => {
    const task = await getTaskReq(id)
    tasks.value.push(task)
  }

  const deleteTask = async (id: number) => {
    await deleteTaskReq(id)
    const index = tasks.value.findIndex((a) => a.id === id)
    if (index !== undefined && index !== -1) {
      tasks.value.splice(index, 1)
    }
  }

  return {
    tasks,
    tasksGroupsByStatus,
    getById,
    listRootTasks,
    listChildTasks,
    createTask,
    getTask,
    deleteTask,
  }
})