# DepMap深度解析：癌症依赖性图谱的构成、数据获取与API应用

DepMap（Cancer Dependency Map，癌症依赖性图谱）是一项具有里程碑意义的科学研究项目，其核心目标是系统性地识别并绘制出癌细胞赖以生存和增殖的基因及分子通路，即所谓的“癌症依赖性”。这些依赖性代表了癌症的潜在致命弱点，为开发新型、高效且靶向性强的抗癌疗法提供了至关重要的线索和理论基础。该项目由博德研究所（Broad Institute）主导，并与桑格研究所（Wellcome Sanger Institute）等多个国际顶尖科研机构紧密合作，通过大规模的基因组筛选，特别是利用CRISPR-Cas9基因编辑技术和RNA干扰（RNAi）技术，在数百种覆盖多种癌症类型的癌细胞系中进行功能缺失性筛选，以确定哪些基因的抑制或敲除会对特定癌细胞的生存和增殖产生显著影响 [[0](https://depmap.org)] [[1](https://www.broadinstitute.org/cancer/cancer-dependency-map)] [[2](https://pmc.ncbi.nlm.nih.gov/articles/PMC7924953)] [[3](https://sumble.com/tech/depmap)]。DepMap项目不仅致力于生成海量的高质量数据，更强调将这些数据资源向全球科研界开放共享，以加速癌症研究的进程，推动个性化医疗的发展，并最终惠及癌症患者 [[0](https://depmap.org)]。本报告旨在深入剖析DepMap数据库的构成，详细阐述其数据的下载途径与方法，并全面介绍其应用程序接口（API）的可用性与功能，为研究人员提供一个关于DepMap资源的综合性指南，以便更好地利用这一强大的平台开展癌症相关研究。

## DepMap数据库的构成：一个多维度、分层次的癌症生物学知识库

DepMap数据库的构建是一个持续演进和不断扩展的过程，它整合了多种类型的组学数据和功能筛选数据，形成了一个复杂而精细的多维度、分层次的知识体系，旨在全面揭示癌细胞的脆弱性。其数据构成的核心在于对大量癌症细胞模型进行系统的基因组表征和功能依赖性分析。这些细胞模型主要来源于癌症细胞系百科全书（Cancer Cell Line Encyclopedia, CCLE）和Project Achilles等早期项目，并在DepMap项目框架下得到了极大的扩展和深化 [[11](https://depmap.org/portal/data_page)]。数据库中的数据主要可以分为两大类：一类是描述癌细胞模型基本基因组特征的表征数据（characterization data），另一类是通过功能筛选实验识别癌细胞依赖性的筛选数据（perturbation data）。这些数据共同构成了DepMap的核心资源，为研究人员提供了从基因组背景到功能表型的全方位视角。

首先，在**表征数据**方面，DepMap整合了多种高通量基因组学技术产生的数据，以全面描绘癌细胞系的分子特征。这包括：
1.  **基因突变数据（Mutation Data）**：通过全外显子组测序（Whole Exome Sequencing, WES）或全基因组测序（Whole Genome Sequencing, WGS）识别的癌细胞系中的体细胞突变，包括单核苷酸变异（SNVs）、小片段插入缺失（Indels）等。这些数据有助于理解癌细胞的驱动基因和潜在的药物靶点。
2.  **基因拷贝数变异数据（Copy Number Alterations, CNA）**：通过 SNP 芯片或测序数据分析获得的基因组区域扩增或缺失信息。基因拷贝数的异常改变是癌症基因组的一个重要特征，直接影响基因的表达水平，并与癌细胞的依赖性密切相关。
3.  **基因表达数据（Gene Expression Data）**：通常通过RNA测序（RNA-Seq）技术测量，提供癌细胞系中基因转录本水平的定量信息。表达谱数据可以用于识别癌症亚型、信号通路活性以及潜在的生物标志物。
4.  **蛋白质组学数据（Proteomics Data）**：例如通过反向相位蛋白质阵列（Reverse Phase Protein Array, RPPA）或质谱（Mass Spectrometry, MS）技术获得的蛋白质表达和磷酸化水平信息。蛋白质是生命功能的直接执行者，蛋白质组学数据能更直接地反映细胞的状态和信号通路活性。
5.  **甲基化数据（Methylation Data）**：检测DNA甲基化修饰模式，特别是启动子区域的甲基化状态，这与基因表达的调控密切相关，在癌症的发生发展中扮演重要角色。
6.  **其他组学数据**：如microRNA表达数据等，进一步丰富了癌细胞模型的分子特征谱 [[11](https://depmap.org/portal/data_page)]。这些表征数据为理解癌细胞的功能依赖性提供了必要的基因组背景信息，并有助于将依赖性数据与特定的分子亚型或通路异常相关联。

其次，在**功能筛选数据**方面，DepMap的核心在于其大规模的基因依赖性筛选，主要采用两种互补的技术：
1.  **CRISPR-Cas9 筛选（CRISPR Screens）**：这是DepMap项目最主要的依赖性识别手段。通过在全基因组范围内设计sgRNA文库，对癌细胞系进行CRISPR-Cas9介导的基因敲除，然后通过高通量测序检测sgRNA的丰度变化，从而评估每个基因对癌细胞生存和增殖的重要性。博德研究所（Broad Institute）和桑格研究所（Wellcome Sanger Institute）均贡献了大量的CRISPR筛选数据 [[11](https://depmap.org/portal/data_page)]。CRISPR筛选以其高特异性和高效性，成为识别癌症必需基因的强大工具。DepMap提供的CRISPR数据通常以基因效应（gene effect）或基因依赖性（gene dependency）分数的形式呈现，这些分数反映了基因敲除对细胞活力的影响程度。
2.  **RNA干扰筛选（RNAi Screens）**：在CRISPR技术广泛应用之前，RNAi（包括shRNA和siRNA）是进行基因功能缺失筛选的主要方法。DepMap也整合了来自不同研究机构（如Novartis的Drive项目，Broad Institute的Achilles项目的早期数据等）的RNAi筛选数据 [[11](https://depmap.org/portal/data_page)]。尽管RNAi技术可能存在脱靶效应和敲除效率不高等问题，但其积累的大量数据仍然为验证CRISPR结果和提供补充信息具有重要价值。

除了基因依赖性筛选，DepMap还整合了**药物敏感性数据（Drug Sensitivity Data）**，例如通过PRISM（Profiling Relative Inhibition Simultaneously in Mixtures）药物筛选平台或与GDSC（Genomics of Drug Sensitivity in Cancer）项目合作获得的数据。这些数据评估了大量化合物（包括已批准药物和实验性化合物）在不同癌细胞系中的抑制效果，有助于将基因依赖性与药物反应联系起来，发现新的治疗机会或预测现有药物的潜在用途 [[11](https://depmap.org/portal/data_page)]。

DepMap数据库的组织结构体现了其复杂性和数据的关联性。它采用了一个多层次的层级结构来管理日益丰富的数据类型，并确保数据的灵活性和可扩展性 [[11](https://depmap.org/portal/data_page)]。这个层级结构从顶层到底层主要包括：
1.  **患者（Patient）**：数据结构的顶层，代表了癌细胞来源的个体。
2.  **模型（Model）**：指从单个患者活检组织衍生出的细胞集合，通常是特定的癌细胞系。每个模型都有一个唯一的模型ID（例如，以"ACH-"开头的ID），并关联着丰富的元数据，如细胞系名称、谱系（lineage）、组织来源等。`Model.csv` 文件提供了模型ID与这些元数据之间的映射关系 [[11](https://depmap.org/portal/data_page)]。
3.  **模型条件（Model Condition）**：指在特定实验条件下对模型进行的处理或培养，例如不同的生长培养基、药物处理等。每个模型条件有一个唯一的模型条件ID（例如，"MC-xxxxxx-yyyy"格式）。`ModelCondition.csv` 文件用于映射模型条件相关的元数据 [[11](https://depmap.org/portal/data_page)]。CRISPR筛选和测序数据通常是在特定的模型条件下产生的。
4.  **筛选（Screen）**：特指一次功能筛选实验，例如一次CRISPR敲除筛选。每个筛选都有一个唯一的筛选ID（Screen ID, 例如"SC-xxxxxx-xxyy"）。`ScreenSequenceMap.csv` 文件提供了筛选ID到模型ID和模型条件ID的映射关系 [[11](https://depmap.org/portal/data_page)]。对于CRISPR数据，存在以"Screen"为前缀的数据文件，它们按筛选ID索引。同时，也存在以"CRISPR"为前缀的数据文件，这些文件将在相同基础模型（Model）下进行的多次筛选结果进行了合并，`CRISPRScreenMap.csv` 文件说明了这些文件中合并了哪些筛选 [[11](https://depmap.org/portal/data_page)]。
5.  **组学谱（Omics Profile）**：指特定类型的组学数据，如WGS、RNA-Seq、WES等。每种组学数据类型都有一个唯一的组学谱ID（Omics Profile ID）。一个组学谱ID可以关联一个或多个测序ID（Sequencing ID, 内部标识符，如"CDS-"开头）。`OmicsProfiles.csv` 文件提供了完整的组学元数据和ID映射信息 [[11](https://depmap.org/portal/data_page)]。组学数据文件通常包含`ModelID`、`ModelConditionID`、`IsDefaultEntryModel`和`IsDefaultEntryMC`等ID列。`IsDefaultEntryForModel`和`IsDefaultEntryForMC`标志用于标识代表特定模型或模型条件的默认测序输出 [[11](https://depmap.org/portal/data_page)]。

DepMap的数据并非一成不变，而是通过定期的数据发布（Data Release）不断更新和扩充。新的DepMap数据通常每年发布两次，分别在五月和十一月 [[11](https://depmap.org/portal/data_page)]。这意味着数据库中包含的细胞系数量、数据类型以及数据质量都在持续提升。除了DepMap自身产生的核心数据集（称为"DepMap Release Datasets"），DepMap门户还托管了由合作者生成或提供的数据集（"Collaborator Datasets"）。这些合作者数据集可能不会持续更新，但经过DepMap的协调处理（harmonized）后，可供科研社区使用，并可能在门户的某些工具中可用 [[11](https://depmap.org/portal/data_page)]。例如，CCLE的原始测序数据（针对2019年发表的文章）可通过Sequence Read Archive (SRA)等途径获取，而处理后的CCLE数据集则主要在DepMap门户的数据页面提供 [[18](https://depmap.org/portal/ccle)]。此外，还有一些特定的项目数据，如MetMap（转移图谱，Metastasis Map）项目，专注于癌细胞的转移潜力，其队列注释数据也通过DepMap平台提供下载 [[19](https://depmap.org/metmap/data)]。

DepMap门户提供了一系列交互式工具（如Data Explorer, Custom Analyses, Celligner, Target Discovery, Context Explorer等），帮助用户浏览、分析和可视化这些复杂的数据 [[11](https://depmap.org/portal/data_page)]。例如，Data Explorer允许用户查看不同数据集在特定细胞系或基因下的数据。这种数据与工具的紧密结合，使得研究人员能够更方便地从DepMap数据库中挖掘有价值的生物学见解。理解DepMap数据库的这种多维度构成和分层次组织结构，对于有效地查询、下载和解读其数据至关重要，它为后续的下载操作和API应用奠定了坚实的基础。DepMap致力于构建一个不断演进的、能够容纳和分析新型数据的灵活结构，以更好地服务于癌症研究社区 [[11](https://depmap.org/portal/data_page)]。

## DepMap数据下载指南：从门户界面到程序化访问

DepMap项目秉承开放科学（Open Science）的精神，致力于将其产生的宝贵数据资源免费提供给全球科研社区使用 [[0](https://depmap.org)]。用户可以通过多种途径获取DepMap数据，主要包括通过DepMap门户网站的用户界面进行手动下载，以及利用DepMap提供的API进行程序化下载。这些数据主要用于研究目的，不被允许直接用于临床或商业用途，例如直接销售、整合到产品中，或用于训练、开发或增强机器学习或AI模型（内部研究使用或非营利性研究共享除外）[[0](https://depmap.org)]。商业用途可能需要从Broad Institute或其贡献者处获得单独的许可协议。用户在使用数据时，必须遵守DepMap的使用条款，包括正确引用数据来源和保护可识别数据主体的机密性 [[0](https://depmap.org)]。

**通过DepMap门户网站下载数据**

DepMap门户网站（[https://depmap.org/portal/data_page](https://depmap.org/portal/data_page)）是用户下载数据的主要入口。该页面提供了清晰的数据组织和多种下载选项，以满足不同用户的需求 [[11](https://depmap.org/portal/data_page)]。主要的数据下载区域包括：

1.  **当前发布版本 (Current Release)**：此部分提供了DepMap最新官方发布的数据集。DepMap数据每年发布两次（通常在五月和十一月），"当前发布版本"页面会列出本次发布包含的所有核心数据文件。用户可以在此找到最新的CRISPR筛选结果、基因表达谱、拷贝数变异、突变数据、药物敏感性数据等。这些数据文件通常以CSV格式提供，方便用户导入到各种数据分析软件中。页面通常会提供每个数据文件的简要描述、文件大小、生成日期以及相关的引用信息。

2.  **自定义下载 (Custom Downloads)**：此功能允许用户根据自己的研究需求，创建并下载定制化的数据子集。用户可以选择特定的数据集，然后根据基因列表（gene symbols）、化合物列表或DepMap细胞系ID（DepMap IDs）进行筛选，从而只下载感兴趣的数据部分。此外，还可以选择是否添加细胞系元数据（如细胞系名称、谱系等），以及是否删除全为NA的行或列 [[11](https://depmap.org/portal/data_page/?tab=customDownloads)]。这种灵活的下载方式可以有效减少下载的数据量，并简化后续的数据处理工作。

3.  **全部数据 (All Data)**：此页面提供了一个全面的、可搜索的DepMap门户中所有可见文件的集合 [[10](https://depmap.org/portal/data_page/?tab=allData)]。用户可以通过下拉菜单选择特定类别的文件集，或者直接按文件名搜索特定的文件。这个页面不仅包含了当前发布版本的数据，还可能包括一些历史版本的数据、合作者提供的数据集以及其他相关资源。每个文件条目通常会提供文件名、描述、数据类型、大小、下载链接以及相关的引用信息和数据生成者信息。例如，在"全部数据"页面，用户可能会找到如`Achilles_gene_effect.csv`（CRISPR基因效应数据）、`CCLE_expression.csv`（CCLE基因表达数据）等核心文件 [[14](https://depmap.org/peddep/datadownload/index.html)]。

除了上述主要下载途径，DepMap门户的特定项目页面也可能提供数据下载。例如，癌症细胞系百科全书（CCLE）的数据在DepMap门户有专门的页面 ([https://depmap.org/portal/ccle](https://depmap.org/portal/ccle))，该页面指出处理后的数据下载可在DepMap门户的数据页面找到，而2019年发表的CCLE文章的原始测序数据则可通过Sequence Read Archive (SRA)获取 [[18](https://depmap.org/portal/ccle)]。同样，MetMap项目的转移潜力队列注释数据也提供了专门的下载链接 [[19](https://depmap.org/metmap/data)]。

在下载和使用DepMap数据时，用户必须注意以下几点：
*   **数据引用**：DepMap团队要求用户在发表基于DepMap数据的研究成果时，正确引用数据来源。对于DepMap发布的数据（包括CRISPR筛选、PRISM药物筛选、拷贝数、突变、表达和融合基因等），应引用相应的Figshare DOI（例如，DepMap, Broad (2025). DepMap Public 25Q3. Dataset. depmap.org，具体版本号需根据实际使用的数据更新）以及DepMap门户网站 ([https://depmap.org/portal](https://depmap.org/portal))。此外，还要求引用DepMap项目的综述性文章，如Arafeh, R., Shibue, T., Dempster, J.M. et al. The present and future of the Cancer Dependency Map. Nat Rev Cancer 25, 59-73 (2025). https://doi.org/10.1038/s41568-024-00763-x [[11](https://depmap.org/portal/data_page)]。对于其他合作者数据集，应在"全部数据下载"页面的相应数据集下拉选项中查找特定的引用信息。
*   **数据许可和用途**：严格遵守DepMap的使用条款，数据仅限于研究目的，禁止未经授权的商业用途。
*   **数据版本**：DepMap数据会定期更新，用户在使用时应注意所下载的数据版本，并在研究中明确说明。

**通过API程序化下载数据**

对于需要批量下载数据、将数据获取集成到自动化分析流程中，或者需要进行复杂数据查询的用户，DepMap提供了应用程序接口（API）。API（Application Programming Interface）是一组定义和协议，允许不同的软件应用程序相互通信。DepMap API为程序化访问其数据和部分门户功能提供了便利。DepMap API的文档和规范可以在 [https://depmap.org/portal/api](https://depmap.org/portal/api) 找到，其Swagger/OpenAPI规范文件位于 [https://depmap.org/portal/api/swagger.json](https://depmap.org/portal/api/swagger.json) [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]。需要强调的是，这些API目前标记为实验性的（experimental），可能会在没有预先通知的情况下发生变化 [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]。

与数据下载相关的API端点主要包括：
*   `/download/files`：此端点允许用户下载一个CSV文件，该文件列出了DepMap门户中所有可供下载的文件及其直接下载链接 [[23](https://forum.depmap.org/t/stable-url-for-current-release-files/3765)] [[40](https://depmap.org/portal/api/swagger.json)]。这对于编写脚本批量下载多个已知文件名的数据集非常有用。
*   `/download/datasets`：此端点返回一个JSON格式的数据集列表，这些数据集可用于自定义下载 [[40](https://depmap.org/portal/api/swagger.json)]。列表中的每个数据集对象通常包含数据集ID (`dataset id`)、显示名称 (`display_name`)、数据类型 (`data_type`) 以及在DepMap下载页面上该数据集信息的URL (`download_entry_url`) [[40](https://depmap.org/portal/api/swagger.json)]。
*   `/download/custom`：此端点允许用户提交一个任务，以下载特定数据集的自定义子集 [[40](https://depmap.org/portal/api/swagger.json)]。用户需要提供数据集ID (`datasetId`)，并可选地提供基因或化合物标签列表 (`featureLabels`)、细胞系DepMap ID列表 (`cellLineIds`)、是否删除全为NA的行列 (`dropEmpty`)、是否添加细胞系元数据 (`addCellLineMetadata`) 等参数。该API调用会返回一个任务ID (`task id`)，用户需要使用此任务ID轮询 `/task/{id}` 端点来获取任务状态。当任务状态为 "SUCCESS" 时，结果中会包含一个 `downloadUrl`，指向生成的自定义数据文件。
*   `/download/custom_merged`：此端点类似于 `/download/custom`，但允许用户下载两个或更多数据集按DepMap ID合并后的单个文件 [[40](https://depmap.org/portal/api/swagger.json)]。用户需要提供一个数据集ID列表 (`datasetIds`)，其他可选参数与 `/download/custom` 类似。同样，此操作也是异步的，返回任务ID供轮询。
*   `/download/custom_mutation_table`：此端点专门用于下载突变表的自定义子集 [[40](https://depmap.org/portal/api/swagger.json)]。用户可以提供基因列表 (`featureLabels`) 和细胞系DepMap ID列表 (`cellLineIds`) 来筛选突变数据。此操作也是异步的。
*   `/download/gene_dep_summary`：此端点允许用户直接下载一个包含所有基因依赖性摘要的CSV文件 [[40](https://depmap.org/portal/api/swagger.json)]。
*   `/download/mutation_table_citation`：此端点用于获取突变表的引用信息URL [[40](https://depmap.org/portal/api/swagger.json)]。

使用这些API端点通常需要编写脚本（例如使用Python的`requests`库）。对于涉及长时间运行的任务（如自定义下载和合并），API采用异步处理机制。用户首先通过POST请求提交任务（如到 `/download/custom`），API返回一个任务对象，包含任务ID (`id`)、状态 (`state`，如 "PENDING", "PROGRESS", "SUCCESS", "FAILURE")、下次轮询延迟 (`nextPollDelay`)、进度百分比 (`percentComplete`) 以及可选的消息 (`message`) 和结果 (`result`) [[40](https://depmap.org/portal/api/swagger.json)]。客户端应用程序需要使用返回的任务ID，定期向 `/task/{id}` 端点发送GET请求以查询任务状态，直到任务完成（状态为 "SUCCESS" 或 "FAILURE"）。如果成功，`result` 字段将包含下载链接或其他相关信息。

此外，DepMap的论坛讨论中也提到，所有DepMap数据最终也会发布在Figshare上，而Figshare本身提供了友好的API，这可能也是程序化下载数据的一个备选途径 [[24](https://forum.depmap.org/t/dataset-download-using-api-call-programmatically/2488)] [[33](https://forum.depmap.org/t/dataset-download-using-api-call-programmatically/2488)]。桑格研究所的DepMap节点 ([https://depmap.sanger.ac.uk](https://depmap.sanger.ac.uk)) 也为其细胞模型护照（Cell Model Passports）等资源提供了独立的HTTP REST API [[4](https://depmap.sanger.ac.uk)] [[16](https://depmap.sanger.ac.uk/documentation/datasets)] [[25](https://depmap.sanger.ac.uk/documentation/api)] [[31](https://depmap.sanger.ac.uk/documentation/api)]。

总而言之，DepMap为用户提供了灵活多样的数据下载方式。无论是通过门户网站的图形界面进行交互式下载，还是利用API进行程序化批量获取，用户都能方便地访问到这一宝贵的癌症依赖性数据资源。选择哪种方式取决于用户的具体需求、数据量以及技术能力。在下载和使用数据时，务必遵守DepMap的条款和引用指南，以支持这一开放科学项目的持续发展。

## DepMap API深度剖析：赋能自动化与可重复性研究

DepMap应用程序接口（API）的引入，极大地提升了研究人员访问、整合和分析DepMap海量癌症依赖性数据的效率与灵活性。API（Application Programming Interface）作为一组预定义的规则和工具，使得软件应用程序能够相互通信和交互，从而实现对DepMap门户功能和数据的程序化访问。DepMap API的设计旨在支持自动化数据获取、定制化数据提取、以及将DepMap数据无缝集成到复杂的生物信息学分析流程中，这对于促进可重复性研究和大规模数据挖掘具有重要意义。DepMap API的官方文档和入口位于 [https://depmap.org/portal/api](https://depmap.org/portal/api)，其详细的规范以OpenAPI（Swagger 2.0）格式提供，可通过 [https://depmap.org/portal/api/swagger.json](https://depmap.org/portal/api/swagger.json) 获取 [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]。正如其文档中明确指出的，当前版本的DepMap API（1.0版）仍处于实验阶段，这意味着其接口和功能在未来可能会发生变更，用户在开发关键应用时需要注意这一点 [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]。尽管如此，DepMap API已经为研究社区提供了一套强大的工具，用于解锁DepMap数据库的深层潜力。

DepMap API的架构遵循RESTful（Representational State Transfer）设计原则，这是一种广泛采用的Web服务架构风格，强调无状态、可缓存、客户端-服务器架构以及统一的接口。RESTful API使用标准的HTTP方法（如GET、POST、PUT、DELETE）来执行操作，并通过URL（统一资源定位符）来标识资源。DepMap API的基础路径（basePath）是 `/portal/api`，所有API端点都是相对于此路径构建的 [[40](https://depmap.org/portal/api/swagger.json)]。API主要消费和产生JSON（JavaScript Object Notation）格式的数据，这是一种轻量级、易于阅读和机器解析的数据交换格式。API的响应通常会包含描述性的HTTP状态码，以指示请求的成功或失败，例如，状态码200表示请求成功。

DepMap API的功能通过一系列“端点”（endpoints）来实现，每个端点对应一个特定的URL路径，并暴露一组预定义的操作。根据Swagger规范，DepMap API主要包含以下几个功能模块（tags）及其对应的端点 [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]：

1.  **`download`**：这是与数据下载最直接相关的模块，提供了多个端点用于程序化获取DepMap数据。
    *   `GET /download/files`：此端点允许用户获取一个CSV格式的文件列表，其中包含了DepMap门户中所有可供下载的文件及其对应的直接下载URL [[23](https://forum.depmap.org/t/stable-url-for-current-release-files/3765)] [[40](https://depmap.org/portal/api/swagger.json)]。这对于需要批量下载大量预定义文件的用户非常有用。
    *   `GET /download/datasets`：此端点返回一个JSON数组，列出了所有可用于自定义下载的数据集。每个数据集对象包含数据集ID、显示名称、数据类型以及在DepMap下载页面上该数据集详细信息的URL [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `POST /download/custom`：此端点用于提交一个自定义数据下载任务。用户可以通过查询参数指定数据集ID (`datasetId`)，以及可选的筛选条件，如基因或化合物标签列表 (`featureLabels`)、细胞系DepMap ID列表 (`cellLineIds`)、是否删除全为NA的行或列 (`dropEmpty`，默认为false)、是否在CSV文件中添加细胞系元数据如名称和谱系 (`addCellLineMetadata`，默认为false) [[40](https://depmap.org/portal/api/swagger.json)]。该请求是异步的，成功提交后会返回一个任务对象（Task object），其中包含任务ID (`id`)、初始状态 (`state`，通常为 "PENDING") 和建议的下次轮询延迟 (`nextPollDelay`)。
    *   `POST /download/custom_merged`：与 `/download/custom` 类似，但此端点允许用户提交一个任务，用于下载两个或更多数据集按DepMap ID合并后的单个文件。用户需要提供一个数据集ID列表 (`datasetIds`)，其他筛选参数与自定义下载类似 [[40](https://depmap.org/portal/api/swagger.json)]。同样返回任务对象供后续轮询。
    *   `POST /download/custom_mutation_table`：此端点专门用于提交一个自定义突变表下载任务。用户可以指定基因列表 (`featureLabels`) 和细胞系DepMap ID列表 (`cellLineIds`) 来筛选突变数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /download/gene_dep_summary`：此端点允许用户直接下载一个包含所有基因依赖性摘要信息的CSV文件，无需提交异步任务 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /download/mutation_table_citation`：此端点用于获取突变表的引用信息URL，需要提供突变数据集在DepMap下载页面的URL作为查询参数 (`download_entry_url`) [[40](https://depmap.org/portal/api/swagger.json)]。

2.  **`task`**：此模块用于管理长时间运行的任务，特别是那些由 `/download/custom*` 端点提交的下载任务。
    *   `GET /task/{id}`：此端点用于查询特定任务（由路径参数 `id` 指定）的状态和结果。返回的任务对象包含任务ID、状态 (`state`，如 "PENDING", "PROGRESS", "SUCCESS", "FAILURE")、下次轮询延迟 (`nextPollDelay`)、进度百分比 (`percentComplete`，对于下载任务可能始终为null)、进度或失败消息 (`message`)，以及当任务成功时 (`state` 为 "SUCCESS") 包含的结果对象 (`result`)。对于下载任务，`result` 通常是一个包含 `downloadUrl` 字段的JSON对象，该字段指向生成的可下载文件 [[40](https://depmap.org/portal/api/swagger.json)]。

3.  **`data_page`**：此模块提供与DepMap门户数据页面相关的信息。
    *   `GET /data_page/data_availability`：获取当前数据发布版本以及整个门户中数据的可用性信息 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /data_page/lineage_availability`：获取整个门户中不同谱系（lineage）数据的可用性情况 [[40](https://depmap.org/portal/api/swagger.json)]。

4.  **`context_explorer`**：此模块提供与DepMap门户中“Context Explorer”工具相关的数据访问功能。Context Explorer是一个交互式工具，允许用户在特定的生物学或实验背景下探索基因依赖性和药物敏感性数据。
    *   `GET /context_explorer/analysis_data`：获取Context Explorer中基因依赖性标签页或药物敏感性标签页的所有数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_box_plot_data`：获取Context Explorer中箱线图的数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_dose_curves`：获取Context Explorer中剂量反应曲线的数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_info`：获取可用的上下文树（context trees）列表，以字典形式返回，键为每个可用的非终端节点，值为从该节点分出的分支 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_node_name`：功能与 `context_info` 类似，获取可用的上下文树节点名称 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_path`：获取上下文路径信息 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_search_options`：获取Context Explorer中的搜索选项 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/context_summary`：获取上下文摘要信息，结构类似 `context_info` [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/enriched_lineages_tile`：获取富集谱系瓦片（tile）的数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /context_explorer/subtype_data_availability`：获取亚型数据可用性信息 [[40](https://depmap.org/portal/api/swagger.json)]。

5.  **`compound`**：此模块提供与化合物（药物）相关的数据访问功能。
    *   `GET /compound/dose_curve_data`：获取化合物的剂量反应曲线数据 [[40](https://depmap.org/portal/api/swagger.json)]。
    *   `GET /compound/prioritized_dataset`：获取优先化的化合物数据集 [[40](https://depmap.org/portal/api/swagger.json)]。

6.  **`dataset_manager`**：此模块用于管理DepMap内部称为"Breadbox"的数据集。Breadbox似乎是DepMap用于存储和管理矩阵型数据集的系统。
    *   `POST /dataset_manager/copy_to_breadbox`：将一个连续矩阵数据集从旧版数据库（legacy database）复制到Breadbox中。一旦复制，门户将默认使用Breadbox版本。此操作需要指定旧版数据集的ID (`dataset_id`)，并可选地指定Breadbox组的UUID (`group_id`) 和Breadbox特征类型 (`feature_type`) [[40](https://depmap.org/portal/api/swagger.json)]。

7.  **`health_check`**：此模块提供检查DepMap门户及其后端服务健康状况的端点。
    *   `GET /health_check/celery_redis_check`：测试Celery任务队列和Redis缓存系统的健康状况。此端点会提交一个测试任务到Celery，并等待其完成，以验证往返通信是否正常工作 [[40](https://depmap.org/portal/api/swagger.json)]。

DepMap API的许多端点支持可选的 `X-Fields` 请求头。这是一个字段掩码（field mask），允许客户端指定希望在响应中返回的字段，从而减少不必要的数据传输，提高API调用的效率 [[40](https://depmap.org/portal/api/swagger.json)]。

**使用DepMap API的示例场景：**

假设一位研究人员想要获取特定一组基因（例如，与DNA修复相关的基因）在特定一组癌细胞系（例如，所有乳腺癌细胞系）中的CRISPR基因效应数据。

1.  **发现数据集**：首先，研究人员可能需要通过 `GET /download/datasets` 端点获取所有可下载数据集的列表，并从中找到包含CRISPR基因效应数据的数据集ID（例如，可能是一个名为"Achilles Gene Effect"的数据集）。
2.  **获取细胞系ID**：研究人员需要确定他们感兴趣的乳腺癌细胞系的DepMap ID。这可能需要先通过门户或其他元数据文件（如`Model.csv`，可通过`/download/files`获取）来查找。
3.  **提交自定义下载任务**：研究人员使用 `POST /download/custom` 端点，在请求体中（或作为查询参数，具体取决于API实现细节，Swagger规范中显示为查询参数）提供以下信息：
    *   `datasetId`: 上一步找到的CRISPR基因效应数据集的ID。
    *   `featureLabels`: 感兴趣的DNA修复相关基因的symbol列表。
    *   `cellLineIds`: 感兴趣的乳腺癌细胞系的DepMap ID列表。
    *   `addCellLineMetadata`: 可能设置为 `true`，以便在下载的CSV中包含细胞系名称和谱系信息。
4.  **轮询任务状态**：API调用会返回一个任务ID。研究人员需要编写一个循环，定期使用 `GET /task/{id}` 端点（将 `{id}` 替换为实际的任务ID）来检查任务状态。
5.  **下载数据**：当任务状态变为 "SUCCESS" 时，响应的 `result` 字段中将包含一个 `downloadUrl`。研究人员可以使用此URL下载生成的CSV文件，该文件即为定制化的CRISPR基因效应数据。

这种程序化的访问方式，使得研究人员可以轻松地将DepMap数据整合到其自动化分析流程中，例如，定期获取最新数据、进行大规模的关联分析、或者将DepMap数据与其他外部数据集进行整合。

**DepMap API的潜力与考量：**

DepMap API为癌症研究社区带来了显著的益处：
*   **自动化与效率**：API使得数据获取和分析过程自动化成为可能，显著提高了研究效率，特别是对于需要处理大量数据或进行重复性任务的研究。
*   **可重复性研究**：通过API，研究方法和数据获取步骤可以被精确地记录和复现，这对于科学研究的可重复性至关重要。
*   **定制化数据提取**：研究人员可以根据具体研究需求，精确提取所需的数据子集，避免了下载和处理不必要的数据。
*   **集成与互操作性**：API促进了DepMap数据与其他生物信息学工具、数据库和计算平台的集成，为更复杂的综合分析铺平了道路。
*   **创新应用开发**：开放API为第三方开发者提供了机会，可以基于DepMap数据构建新的应用程序、可视化工具或分析服务。

然而，在使用DepMap API时，也需要考虑一些因素：
*   **实验性状态**：API目前标记为实验性，这意味着接口可能发生变化。用户需要关注API的更新，并相应地调整其应用程序。
*   **异步处理**：对于耗时的操作（如大数据集的自定义下载），API采用异步处理机制。客户端需要正确实现任务轮询逻辑。
*   **速率限制**：虽然当前的Swagger规范中没有明确提及，但许多公共API都有速率限制以防止滥用。开发者在设计高频次调用的应用时应考虑到这一点。
*   **错误处理**：API调用可能会因为各种原因失败（如无效参数、服务器错误等）。客户端代码需要能够妥善处理这些错误情况。
*   **认证与授权**：目前DepMap API的端点似乎不需要特殊的认证即可访问（基于Swagger规范和论坛讨论）。然而，对于未来更复杂的功能或对特定敏感数据的访问，可能会引入认证机制。

DepMap在GitHub上还提供了一个名为 `broadinstitute/depmap-api` 的仓库 ([https://github.com/broadinstitute/depmap-api](https://github.com/broadinstitute/depmap-api)) [[21](https://github.com/broadinstitute/depmap-api)]。虽然当前对该仓库的观察主要显示了其行为准则，但这类仓库通常用于存放API客户端库示例、SDK（软件开发工具包）或更详细的开发者文档，值得研究人员关注。

此外，桑格研究所的DepMap节点 ([https://depmap.sanger.ac.uk](https://depmap.sanger.ac.uk)) 也为其“细胞模型护照”（Cell Model Passports）提供了独立的REST API [[25](https://depmap.sanger.ac.uk/documentation/api)] [[31](https://depmap.sanger.ac.uk/documentation/api)]。该API允许用户通过HTTP GET请求获取细胞模型的信息，为访问桑格研究所的DepMap相关资源提供了程序化途径。

综上所述，DepMap API是一个功能强大且不断发展的工具集，它极大地增强了DepMap数据的可访问性和可用性。通过提供程序化访问途径，API不仅简化了数据获取过程，还为自动化分析、可重复性研究和创新应用的开发奠定了基础。随着DepMap项目的持续发展和API的不断完善，它必将在推动癌症研究和药物发现方面发挥越来越重要的作用。研究人员应积极探索和利用DepMap API，以充分挖掘这一宝贵数据资源的潜力。

## 结论：DepMap作为癌症研究的基石与未来展望

DepMap（癌症依赖性图谱）项目无疑是现代癌症研究领域的一项里程碑式成就。它通过系统性地识别和绘制癌细胞赖以生存的基因及分子通路，为全球科研界提供了一个前所未有的、内容丰富的、且持续更新的资源库。本报告深入剖析了DepMap数据库的复杂构成，详细阐述了其多样化的数据下载途径，并全面介绍了其应用程序接口（API）的功能与应用，旨在为研究人员提供一个清晰、全面的DepMap资源利用指南。

DepMap数据库的构成体现了其深度与广度。它整合了来自数百种癌症细胞系的多维度组学数据，包括基因突变、拷贝数变异、基因表达、蛋白质组学和甲基化等表征数据，以及通过大规模CRISPR-Cas9和RNAi筛选产生的功能依赖性数据 [[11](https://depmap.org/portal/data_page)]。这些数据并非孤立存在，而是通过一个精心设计的、多层次的层级结构（患者 -> 模型 -> 模型条件 -> 筛选/组学谱）进行组织和管理，确保了数据的关联性、可追溯性和可扩展性 [[11](https://depmap.org/portal/data_page)]。DepMap数据每年两次的定期发布，以及对合作者数据集的整合，进一步保证了数据库的时效性和全面性 [[11](https://depmap.org/portal/data_page)]。这种丰富的数据构成和精细的组织结构，使得研究人员能够从基因组背景到功能表型，全方位地理解癌症的脆弱性，为发现新的药物靶点、理解耐药机制、以及开发个性化治疗策略提供了坚实的基础。

在数据获取方面，DepMap项目充分体现了其对开放科学（Open Science）的承诺 [[0](https://depmap.org)]。通过其用户友好的门户网站 ([https://depmap.org/portal/data_page](https://depmap.org/portal/data_page))，研究人员可以方便地浏览、查询和下载数据。无论是获取最新发布的完整数据集（Current Release），还是根据特定研究需求创建定制化的数据子集（Custom Downloads），亦或是访问门户中所有可用的文件（All Data），DepMap都提供了直观的界面和灵活的选项 [[10](https://depmap.org/portal/data_page/?tab=allData)] [[11](https://depmap.org/portal/data_page)] [[13](https://depmap.org/portal/data_page/?tab=customDownloads)] [[15](https://depmap.org/portal/data_page/?tab=currentRelease)]。同时，DepMap对数据的引用和使用做出了明确规定，确保了数据的合规使用和学术贡献的认可 [[0](https://depmap.org)] [[11](https://depmap.org/portal/data_page)]。

更为重要的是，DepMap提供的应用程序接口（API） ([https://depmap.org/portal/api](https://depmap.org/portal/api)) [[20](https://depmap.org/portal/api)] [[40](https://depmap.org/portal/api/swagger.json)]，为数据的程序化访问和自动化分析打开了大门。这套基于RESTful架构的API，尽管目前仍处于实验阶段，但已经展现出强大的功能。通过一系列定义明确的端点，研究人员可以实现从获取数据集列表、提交自定义下载任务、查询任务状态，到最终获取数据下载链接的完整流程。特别是对于大规模数据处理、重复性分析任务以及将DepMap数据整合到复杂的生物信息学流程中，API提供了无与伦比的效率和灵活性。异步任务处理机制确保了即使是资源密集型的操作也能得到有效管理。API的引入不仅提升了数据访问的便捷性，更是推动了研究的可重复性和创新应用的开发，使得DepMap数据能够更广泛、更深入地应用于癌症研究的各个层面。

展望未来，DepMap项目及其数据库仍有巨大的发展潜力。首先，随着单细胞组学、空间组学等新兴技术的成熟和应用，DepMap有望整合更高分辨率、更精细维度的数据，从而揭示癌细胞异质性、肿瘤微环境对依赖性的影响等更深层次的生物学问题。其次，人工智能（AI）和机器学习（ML）技术在处理和解读大规模复杂数据方面具有独特优势，虽然DepMap的原始数据不直接允许用于商业AI/ML模型的训练增强（内部研究除外）[[0](https://depmap.org)]，但科研界可以利用这些数据开发新的算法和模型，以预测新的癌症依赖性、识别生物标志物、优化药物组合等。DepMap API的进一步完善和稳定化，将为这类计算驱动的发现提供更强大的支持。此外，将DepMap的依赖性数据与临床数据（如患者治疗反应、生存数据等）进行更紧密的整合，将是推动精准肿瘤学发展的关键。尽管DepMap数据本身主要来源于细胞系模型，与临床直接应用尚有距离，但通过建立细胞系模型与原代肿瘤、类器官（organoids）以及患者衍生异种移植（PDX）模型之间的关联桥梁，DepMap的发现将更具临床转化价值。同时，加强不同DepMap节点（如Broad Institute和Sanger Institute）之间数据标准和API的互操作性，以及与其他大型癌症基因组项目（如TCGA，ICGC）的数据整合，将构建一个更全面、更强大的全球癌症研究生态系统。

然而，挑战与机遇并存。如何持续保证数据的质量和标准化，如何有效管理和解读日益增长的数据复杂性，如何将基于细胞系的研究发现有效地转化到临床应用，这些都是DepMap项目未来需要不断面对和解决的问题。API的“实验性”标签也提示我们需要关注其稳定性和向后兼容性。

总而言之，DepMap数据库及其相关的数据获取和访问工具，已经成为癌症研究中不可或缺的基石。它不仅极大地加速了我们对癌症生物学机制的理解，也为新药研发和治疗策略的优化提供了宝贵的线索。通过本报告对DepMap数据库构成、下载方法和API应用的深入探讨，我们期望能够帮助研究人员更有效地利用这一强大的资源，共同为攻克癌症这一全球性挑战贡献力量。随着技术的进步和科研社区的共同努力，DepMap必将在未来的癌症研究和精准医疗中发挥更加核心和深远的影响。

# 参考文献

[0] DepMap: The Cancer Dependency Map Project at Broad. https://depmap.org.

[1] Cancer Dependency Map. https://www.broadinstitute.org/cancer/cancer-dependency-map.

[2] shinyDepMap, a tool to identify targetable cancer genes. https://pmc.ncbi.nlm.nih.gov/articles/PMC7924953.

[3] What is DepMap? Competitors, Complementary Techs &. https://sumble.com/tech/depmap.

[4] Cancer Dependency Map - Wellcome Sanger Institute. https://depmap.sanger.ac.uk.

[10] All Data Downloads. https://depmap.org/portal/data_page/?tab=allData.

[11] Data | DepMap Portal. https://depmap.org/portal/data_page.

[13] Create and download a customized dataset. https://depmap.org/portal/data_page/?tab=customDownloads.

[14] Downloads. https://depmap.org/peddep/datadownload/index.html.

[15] Data | DepMap Portal. https://depmap.org/portal/data_page/?tab=currentRelease.

[16] Datasets - The Cancer Dependency Map at Sanger. https://depmap.sanger.ac.uk/documentation/datasets.

[18] Cancer Cell Line Encyclopedia (CCLE). https://depmap.org/portal/ccle.

[19] MetMap pan-cancer cohort annotation. https://depmap.org/metmap/data.

[20] DepMap APIs. https://depmap.org/portal/api.

[21] broadinstitute/depmap-api. https://github.com/broadinstitute/depmap-api.

[23] Stable URL for current release files - Q&A. https://forum.depmap.org/t/stable-url-for-current-release-files/3765.

[24] Dataset download using API call programmatically - Q&A. https://forum.depmap.org/t/dataset-download-using-api-call-programmatically/2488.

[25] API - The Cancer Dependency Map at Sanger. https://depmap.sanger.ac.uk/documentation/api.

[31] API - The Cancer Dependency Map at Sanger. https://depmap.sanger.ac.uk/documentation/api.

[33] Dataset download using API call programmatically - Q&A. https://forum.depmap.org/t/dataset-download-using-api-call-programmatically/2488.

[40] swagger.json. https://depmap.org/portal/api/swagger.json.