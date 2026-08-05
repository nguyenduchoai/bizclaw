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
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import vn.bizclaw.agent.BizClawApp

@Composable
fun KnowledgeScreen(app: BizClawApp) {
    var title by remember { mutableStateOf("") }
    var body by remember { mutableStateOf("") }
    var productName by remember { mutableStateOf("") }
    var productPrice by remember { mutableStateOf("") }
    var productStock by remember { mutableStateOf("") }
    var productNote by remember { mutableStateOf("") }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            SectionCard("Bảng giá") {
                Text(
                    "Agent báo giá và tính tiền đơn hàng từ đúng bảng này. " +
                        "Sản phẩm hết hàng thì nó nói thật là hết chứ không nhận đơn.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = productName,
                    onValueChange = { productName = it },
                    label = { Text("Tên sản phẩm") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = productPrice,
                        onValueChange = { productPrice = it.filter(Char::isDigit) },
                        label = { Text("Giá (₫)") },
                        modifier = Modifier.weight(1f),
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                    OutlinedTextField(
                        value = productStock,
                        onValueChange = { productStock = it.filter(Char::isDigit) },
                        label = { Text("Tồn kho") },
                        modifier = Modifier.weight(1f),
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                }
                OutlinedTextField(
                    value = productNote,
                    onValueChange = { productNote = it },
                    label = { Text("Ghi chú (size, màu, chất liệu...)") },
                    modifier = Modifier.fillMaxWidth(),
                )
                Button(
                    onClick = {
                        app.products.save(
                            name = productName,
                            price = productPrice.toLongOrNull() ?: 0L,
                            stock = productStock.toIntOrNull() ?: 0,
                            note = productNote,
                        )
                        productName = ""
                        productPrice = ""
                        productStock = ""
                        productNote = ""
                    },
                    enabled = productName.isNotBlank() && productPrice.isNotBlank(),
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Thêm sản phẩm") }
            }
        }

        items(app.products.products, key = { it.id }) { product ->
            ElevatedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(product.name, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                        StatusPill(
                            if (product.inStock) "còn ${product.stock}" else "hết hàng",
                            if (product.inStock) Ok else Bad,
                        )
                    }
                    Text(product.priceLabel, style = MaterialTheme.typography.bodyMedium)
                    if (product.note.isNotBlank()) {
                        Text(
                            product.note,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    OutlinedButton(onClick = { app.products.delete(product.id) }) { Text("Xoá") }
                }
            }
        }

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
