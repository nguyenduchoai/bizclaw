package vn.bizclaw.agent.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import vn.bizclaw.agent.BizClawApp

@Composable
fun KnowledgeScreen(app: BizClawApp) {
    var title by remember { mutableStateOf("") }
    var body by remember { mutableStateOf("") }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            SectionCard("Thông tin cửa hàng") {
                Text(
                    "Agent chỉ được trả lời dựa trên những gì nạp ở đây. Thiếu thông tin " +
                        "thì nó chuyển việc lại cho anh thay vì bịa.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Tiêu đề") },
                    placeholder = { Text("Bảng giá / Phí ship / Bảo hành") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = body,
                    onValueChange = { body = it },
                    label = { Text("Nội dung") },
                    placeholder = { Text("Ship nội thành HN 25k, ngoại thành 35k, đơn trên 500k free...") },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 5,
                )
                Button(
                    onClick = {
                        app.knowledge.save(title, body)
                        title = ""
                        body = ""
                    },
                    enabled = body.isNotBlank(),
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Lưu") }
            }
        }

        item {
            SectionCard("Cách xưng hô") {
                OutlinedTextField(
                    value = app.settings.businessName,
                    onValueChange = { app.settings.businessName = it },
                    label = { Text("Tên cửa hàng") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = app.settings.persona,
                    onValueChange = { app.settings.persona = it },
                    label = { Text("Giọng điệu") },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 2,
                )
            }
        }

        items(app.knowledge.docs, key = { it.id }) { doc ->
            ElevatedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(doc.title, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                        OutlinedButton(onClick = { app.knowledge.delete(doc.id) }) { Text("Xoá") }
                    }
                    Text(doc.body, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }

        if (app.knowledge.docs.isEmpty()) {
            item { EmptyCard("Chưa có tài liệu nào.") }
        }
    }
}
