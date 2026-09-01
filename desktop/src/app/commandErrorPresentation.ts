type StructuredCommandError = {
  code?: unknown;
  safeMessage?: unknown;
};

const actionableMessages: Record<string, string> = {
  contextBudgetInvalid: "当前模型的上下文预算不足。请缩短本次输入或减少输出长度后重试；MindScape 未创建残留节点或运行。",
  credentialNotFound: "尚未配置该 Provider 的 API Key，请先在模型设置中安全保存凭据。",
  credentialStoreUnavailable: "操作系统安全凭据服务当前不可用，请重启 MindScape 或检查系统凭据服务。",
  providerAuthentication: "Provider 拒绝了当前 API Key，请在模型设置中替换凭据并重新测试连接。",
  providerInvalidRequest: "Provider 拒绝了本次运行参数。请检查运行档案中的 reasoning、temperature/top_p、输出格式或预算后重试。",
  providerNetwork: "无法连接 Provider，请检查网络、代理和端点后重新测试连接。",
  providerTimeout: "Provider 连接超时，请检查网络后重试；本次没有自动发起计费请求。",
};

export function commandErrorMessage(error: unknown) {
  if (typeof error === "object" && error !== null) {
    const structured = error as StructuredCommandError;
    if (typeof structured.code === "string" && actionableMessages[structured.code]) {
      return actionableMessages[structured.code];
    }
    if (typeof structured.safeMessage === "string") return structured.safeMessage;
  }
  return error instanceof Error ? error.message : String(error);
}
