<template>
  <div class="not-found-page">
    <el-result icon="warning" title="404" sub-title="页面不存在">
      <template #extra>
        <el-button type="primary" @click="$router.push('/')">返回首页</el-button>
        <el-button @click="$router.back()">返回上页</el-button>
      </template>
    </el-result>
  </div>
</template>

<style scoped>
.not-found-page {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  width: 100%;
  min-width: 0;
  padding: clamp(8px, 3vw, 28px);
}
.not-found-page :deep(.el-result__extra) {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
}
.not-found-page :deep(.el-result__extra .el-button) {
  margin-left: 0;
}
</style>
