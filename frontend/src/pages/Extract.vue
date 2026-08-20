<script setup lang="ts">
import { useRoute } from 'vue-router'
import ExtractionPanel from '@/components/extraction/ExtractionPanel.vue'

const route = useRoute()
const projectId = route.params.id as string
</script>

<template>
  <div>
    <ExtractionPanel :project-id="projectId" />
  </div>
</template>
