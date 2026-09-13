package com.wangxingchao.novel

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

/**
 * 「正在生成」前台服务。
 *
 * 为什么需要它：Android 会把退到后台的应用进程**冻结**（cached 状态），
 * 于是 App 里的内嵌引擎跟着停摆——AI 生成跑到一半就断了。
 * 屏幕常亮只能防息屏，防不住用户切到别的应用。
 *
 * 前台服务（带常驻通知）让进程保持 active，系统不会冻结它。
 * 生成开始时启动、结束或失败时停止（前端负责调用，见 app.js 的 send()）。
 *
 * 注意：
 *   - Android 8+ 必须先建通知渠道，否则 startForeground 会失败
 *   - Android 13+ 需要 POST_NOTIFICATIONS 运行时权限；没有权限时通知不显示，
 *     但服务本身仍可运行（不会崩）
 *   - 清单里声明了 foregroundServiceType="dataSync"（Android 14+ 要求）
 */
class KeepAliveService : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForeground(NOTIFICATION_ID, buildNotification())
    // 被系统回收后重建（生成期间尽量别断）
    return START_STICKY
  }

  private fun buildNotification(): Notification {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        CHANNEL_ID,
        "写作进行中",
        NotificationManager.IMPORTANCE_LOW
      ).apply {
        description = "AI 生成期间保持应用存活，生成结束会自动消失"
        setShowBadge(false)
      }
      val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
      manager.createNotificationChannel(channel)
    }

    return NotificationCompat.Builder(this, CHANNEL_ID)
      .setContentTitle("正在生成")
      .setContentText("生成期间请不要关闭本应用")
      .setSmallIcon(android.R.drawable.stat_notify_sync)
      .setOngoing(true)
      .setPriority(NotificationCompat.PRIORITY_LOW)
      .build()
  }

  companion object {
    const val CHANNEL_ID = "novel_keepalive"
    const val NOTIFICATION_ID = 1001

    fun start(context: Context) {
      val intent = Intent(context, KeepAliveService::class.java)
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        context.startForegroundService(intent)
      } else {
        context.startService(intent)
      }
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, KeepAliveService::class.java))
    }
  }
}
